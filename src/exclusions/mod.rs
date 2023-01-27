use crate::metric::Metric;

pub(crate) trait Exclusion<T>
where
    T: Metric,
{
    fn is_in(&self, point: &T) -> bool;
    fn must_be_in(&self, point: &T, threshold: f64) -> bool;
    fn must_be_out(&self, point: &T, threshold: f64) -> bool;
}

pub(crate) struct BallExclusion<T> {
    pub(crate) point: T,
    pub(crate) radius: f64,
}

impl<T> BallExclusion<T>
where
    T: Metric,
{
    pub(crate) fn new(point: T, radius: f64) -> Self {
        Self { point, radius }
    }
}

impl<T> Exclusion<T> for BallExclusion<T>
where
    T: Metric,
{
    fn is_in(&self, point: &T) -> bool {
        self.point.distance(point) < self.radius
    }

    fn must_be_in(&self, point: &T, threshold: f64) -> bool {
        self.point.distance(point) < (self.radius - threshold)
    }

    fn must_be_out(&self, point: &T, threshold: f64) -> bool {
        self.point.distance(point) >= (self.radius + threshold)
    }
}

// todo: this is only 3p
pub(crate) struct SheetExclusion<T> {
    a: T,
    b: T,
    offset: f64,
}

impl<T> SheetExclusion<T>
where
    T: Metric,
{
    pub(crate) fn new(a: T, b: T, offset: f64) -> Self {
        Self { a, b, offset }
    }
}

impl<T> Exclusion<T> for SheetExclusion<T>
where
    T: Metric,
{
    fn is_in(&self, point: &T) -> bool {
        self.a.distance(point) - self.b.distance(point) - self.offset < 0.0
    }

    fn must_be_in(&self, point: &T, threshold: f64) -> bool {
        todo!()
    }

    fn must_be_out(&self, point: &T, threshold: f64) -> bool {
        todo!()
    }
}
