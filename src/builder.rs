use crate::{metric::Metric, BitPart};

#[cfg(feature = "rayon")]
use crate::exclusions::ExclusionSync;

#[cfg(feature = "rayon")]
use crate::parallel::ParallelBitPart;

/// Builder for a BitPart query.
#[derive(Debug, Clone)]
pub struct BitPartBuilder<T> {
    pub(crate) dataset: Vec<T>,

    pub(crate) radius_increment: f64,
    pub(crate) mean_distance: f64,

    pub(crate) four_point: bool,
    pub(crate) ref_points: u64,
}

impl<T> BitPartBuilder<T>
where
    for<'a> T: Metric + 'a,
{
    /// Create a new `BitPartBuilder` from a dataset.
    pub fn new(dataset: impl IntoIterator<Item = T>) -> Self {
        let dataset = dataset.into_iter().collect::<Vec<_>>();

        Self {
            dataset,
            mean_distance: 1.81,   // TODO: Number was copied from ref impl, why?
            radius_increment: 0.3, // TODO: Number was copied from ref impl, why?
            four_point: true,      // TODO: What does this do?
            ref_points: 40,        // TODO: What if there are fewer than 40 points?
        }
    }

    /// Set the mean distance.
    pub fn mean_distance(mut self, mean_distance: f64) -> Self {
        self.mean_distance = mean_distance;
        self
    }

    /// Set the radius increment.
    pub fn radius_increment(mut self, radius_increment: f64) -> Self {
        self.radius_increment = radius_increment;
        self
    }

    /// Set whether to use four-point or three-point method for sheet exclusions.
    pub fn four_point(mut self, four_point: bool) -> Self {
        self.four_point = four_point;
        self
    }

    /// Set the number of ref points
    /// # Panics
    /// This function will panic if `ref_points` is greater than the size of the dataset.
    pub fn ref_points(mut self, ref_points: u64) -> Self {
        if ref_points as usize > self.dataset.len() {
            panic!("ref_points greater than size of dataset");
        }

        self.ref_points = ref_points;
        self
    }

    /// Build the BitPart.
    pub fn build<'a>(self) -> BitPart<'a, T> {
        BitPart::setup(self)
    }
}

#[cfg(feature = "rayon")]
impl<T> BitPartBuilder<T>
where
    for<'a> T: Metric + Send + Sync + 'a,
    dyn ExclusionSync<T>: Send + Sync,
{
    /// Build the ParallelBitPart.
    /// `job_size` sets the size of each job when performing a range query. For example, `Some(N)` means that
    /// each task will perform `N` bitvector operations sequentially before returning the results.
    /// `None` will disable parallelism during queries altogether - this is useful for small datasets where you only wish
    /// to parallelise the bitset creation.
    ///
    /// `job_size` is related to the concept of "coarseness"; the higher `job_size` is, the more coarse the parallelism is.
    pub fn build_parallel<'a>(self, job_size: Option<u64>) -> ParallelBitPart<'a, T> {
        ParallelBitPart::setup(self, job_size)
    }
}
