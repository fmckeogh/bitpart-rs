use crate::builder::BitPartBuilder;
use crate::exclusions::{BallExclusion, ExclusionSync, SheetExclusion};
use crate::metric::Metric;
use bitvec::prelude::*;
use itertools::Itertools;
use rayon::prelude::*;

pub struct ParallelBitPart<'a, T> {
    dataset: Vec<T>,
    exclusions: Vec<Box<dyn ExclusionSync<T> + Send + Sync + 'a>>,
    bitset: Vec<BitVec>,
    job_size: Option<usize>,
}

impl<'a, T> ParallelBitPart<'a, T>
where
    T: Metric + Send + Sync,
    dyn ExclusionSync<T>: Send + Sync + 'a,
{
    pub fn range_search(&self, point: T, threshold: f64) -> Vec<(T, f64)> {
        let mut ins = vec![];
        let mut outs = vec![];

        for (idx, ez) in self.exclusions.iter().enumerate() {
            if ez.must_be_in(&point, threshold) {
                ins.push(idx);
            } else if ez.must_be_out(&point, threshold) {
                outs.push(idx);
            }
        }

        match (ins.len(), outs.len()) {
            // No exclusions at all, linear search
            (0, 0) => self
                .dataset
                .par_iter()
                .cloned()
                .map(|pt| (pt.clone(), pt.distance(&point)))
                .filter(|(_, dist)| *dist < threshold)
                .collect(),
            // nots, flip, filter
            (0, _) => {
                let nots = self.get_nots(&outs);
                let nots = !nots; // TODO: is nots always of length self.dataset.len()?
                self.filter_contenders(threshold, point, nots)
            }
            // filter
            (_, 0) => {
                let ands = self.get_ands(&ins);
                self.filter_contenders(threshold, point, ands)
            }
            // nots, flip, and, filter
            (_, _) => {
                let ands = self.get_ands(&ins);
                let nots = self.get_nots(&outs);
                let nots = !nots;
                let ands = ands & nots;
                self.filter_contenders(threshold, point, ands)
            }
        }
    }

    /// Performs a bitwise-or on all exclusion zone columns that do not contain the query point.
    fn get_nots(&self, outs: &[usize]) -> BitVec {
        outs.par_chunks(
            self.job_size
                .unwrap_or_else(|| outs.as_parallel_slice().len()),
        )
        .map(|ck| {
            ck.iter()
                .map(|&i| self.bitset.get(i).unwrap())
                .cloned()
                .reduce(|acc, bv| acc | bv)
                .unwrap()
        })
        .reduce(
            || BitVec::repeat(false, self.dataset.len()),
            |acc, bv| acc | bv,
        )
    }

    /// Performs a bitwise-and on all exclusion zone columns that contain the query point.
    fn get_ands(&self, ins: &[usize]) -> BitVec {
        ins.par_chunks(
            self.job_size
                .unwrap_or_else(|| ins.as_parallel_slice().len()),
        )
        .map(|ck| {
            ck.iter()
                .map(|&i| self.bitset.get(i).unwrap())
                .cloned()
                .reduce(|acc, bv| acc & bv)
                .unwrap()
        })
        .reduce(
            || BitVec::repeat(true, self.dataset.len()),
            |acc, bv| acc & bv,
        )
    }

    fn filter_contenders(&self, threshold: f64, point: T, res: BitVec) -> Vec<(T, f64)> {
        res.iter_ones()
            .map(|i| self.dataset.get(i).unwrap())
            .map(|pt| (pt.clone(), pt.distance(&point)))
            .filter(|(_, dist)| *dist <= threshold)
            .collect()
    }

    pub(crate) fn setup(builder: BitPartBuilder<T>, job_size: Option<u64>) -> Self {
        // TODO: actually randomise this
        let ref_points = &builder.dataset[0..(builder.ref_points as usize)];
        let mut exclusions = Self::ball_exclusions(&builder, ref_points);
        exclusions.extend(Self::sheet_exclusions(&builder, ref_points));
        let bitset = Self::make_bitset(&builder, &exclusions);
        Self {
            dataset: builder.dataset,
            bitset,
            exclusions,
            job_size: job_size.map(|n| n as usize),
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
        exclusions
            .par_iter()
            .map(|ex| {
                builder
                    .dataset
                    .iter()
                    .map(|pt| ex.is_in(pt))
                    .collect::<BitVec>()
            })
            .collect::<Vec<_>>()
    }
}