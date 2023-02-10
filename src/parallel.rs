use crate::builder::BitPartBuilder;
use crate::exclusions::{BallExclusion, ExclusionSync, SheetExclusion};
use crate::metric::Metric;
use bitvec::prelude::*;
use itertools::{Either, Itertools};
use rayon::prelude::*;

pub struct ParallelBitPart<'a, T> {
    dataset: Vec<T>,
    exclusions: Vec<Box<dyn ExclusionSync<T> + Send + Sync + 'a>>,
    bitset: Vec<BitVec>,
}

impl<'a, T> ParallelBitPart<'a, T>
where
    T: Metric + Send + Sync,
    dyn ExclusionSync<T>: Send + Sync + 'a,
{
    pub fn range_search(&self, point: T, threshold: f64) -> Vec<(T, f64)> {
        let (ins, outs): (Vec<usize>, Vec<usize>) = self
            .exclusions
            .par_iter()
            .enumerate()
            .filter_map(|(idx, ez)| {
                if ez.must_be_in(&point, threshold) {
                    Some(Either::Left(idx))
                } else if ez.must_be_out(&point, threshold) {
                    Some(Either::Right(idx))
                } else {
                    None
                }
            })
            .partition_map(|x| x);

        self.bitset
            .par_iter()
            .enumerate()
            .filter(|(_, bv)| {
                // If a point is in all `ins`...
                let all_ins = ins.iter().all(|&idx| *bv.get(idx).unwrap());
                // ...and does not show up in any `out`, it is a candidate.
                let never_outs = !outs.iter().any(|&idx| *bv.get(idx).unwrap());
                all_ins & never_outs
            })
            .map(|(idx, _)| self.dataset.get(idx).unwrap())
            .map(|pt| (pt.clone(), point.distance(pt)))
            .filter(|(_, d)| *d <= threshold)
            .collect::<Vec<_>>()
    }

    pub(crate) fn setup(builder: BitPartBuilder<T>) -> Self {
        // TODO: actually randomise this
        let ref_points = &builder.dataset[0..(builder.ref_points as usize)];
        let mut exclusions = Self::ball_exclusions(&builder, ref_points);
        exclusions.extend(Self::sheet_exclusions(&builder, ref_points));
        let bitset = Self::make_bitset(&builder, &exclusions);
        Self {
            dataset: builder.dataset,
            bitset,
            exclusions,
        }
    }

    fn ball_exclusions(
        builder: &BitPartBuilder<T>,
        ref_points: &[T],
    ) -> Vec<Box<dyn ExclusionSync<T> + Send + Sync + 'a>> {
        let radii = [
            builder.mean_distance - 2.0 * builder.radius_increment,
            builder.mean_distance - builder.radius_increment,
            builder.mean_distance,
            builder.mean_distance + builder.radius_increment,
            builder.mean_distance + 2.0 * builder.radius_increment,
        ];

        ref_points
            .iter()
            .cartesian_product(radii.into_iter())
            .map(|(point, radius)| {
                Box::new(BallExclusion::new(point.clone(), radius))
                    as Box<dyn ExclusionSync<T> + Send + Sync>
            })
            .collect()
    }

    fn sheet_exclusions(
        _builder: &BitPartBuilder<T>,
        ref_points: &[T],
    ) -> Vec<Box<dyn ExclusionSync<T> + Send + Sync + 'a>> {
        ref_points
            .iter()
            .combinations(2)
            .map(|x| {
                Box::new(SheetExclusion::new(x[0].clone(), x[1].clone(), 0.0))
                    as Box<dyn ExclusionSync<T> + Send + Sync>
            })
            .collect()
    }

    fn make_bitset(
        builder: &BitPartBuilder<T>,
        exclusions: &[Box<dyn ExclusionSync<T> + Send + Sync + 'a>],
    ) -> Vec<BitVec> {
        // Index by row first
        builder
            .dataset
            .par_iter()
            .map(|pt| exclusions.iter().map(|ez| ez.is_in(pt)).collect::<BitVec>())
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use crate::metric::euclidean::Euclidean;
    use sisap_data::{colors::parse_colors, nasa::parse_nasa};

    use super::*;

    pub(crate) const NASA: &str = include_str!("../sisap-data/src/nasa.ascii");
    pub(crate) const COLORS: &str = include_str!("../sisap-data/src/colors.ascii");

    fn test<T>(dataset: Vec<T>, bitpart: ParallelBitPart<T>, query: T, threshold: f64)
    where
        for<'a> T: Metric + Send + Sync + 'a,
    {
        let res = bitpart.range_search(query.clone(), threshold);

        // Check all points within threshold
        assert!(res
            .iter()
            .all(|(point, _)| point.distance(&query) <= threshold));

        // Check results match up with linear search
        let brute_force = dataset
            .into_iter()
            .map(|pt| pt.distance(&query))
            .filter(|d| *d < threshold)
            .count();

        assert_eq!(res.len(), brute_force);
    }

    #[test]
    fn sisap_nasa_par() {
        let nasa = parse_nasa(NASA)
            .unwrap()
            .into_iter()
            .map(Euclidean::new)
            .collect::<Vec<_>>();

        let bitpart = BitPartBuilder::new(nasa.clone()).build_parallel();
        let query = nasa[317].clone();
        let threshold = 1.0;

        test(nasa, bitpart, query, threshold);
    }

    #[test]
    fn sisap_colors_par() {
        let colors = parse_colors(COLORS)
            .unwrap()
            .into_iter()
            .map(Euclidean::new)
            .collect::<Vec<_>>();

        let bitpart = BitPartBuilder::new(colors.clone()).build_parallel();
        let query = colors[70446].clone();
        let threshold = 0.5;

        test(colors, bitpart, query, threshold);
    }
}
