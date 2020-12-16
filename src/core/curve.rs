//! Curve used for interpolation (e.g. gradients)
use crate::geom2::Vector2f;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait CurveNode:
    Clone
    + std::ops::Mul<f32, Output = Self>
    + std::ops::Add<Output = Self>
    + Debug
    + std::ops::Sub<Output = Self>
{
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Curve<T>
where
    T: CurveNode,
{
    pub(crate) xs: Vec<f32>,
    pub(crate) ys: Vec<T>,
}

impl<T> Default for Curve<T>
where
    T: CurveNode,
{
    fn default() -> Self {
        Self {
            xs: vec![],
            ys: vec![],
        }
    }
}

impl<T> Curve<T>
where
    T: CurveNode,
{
    pub fn y(&self, t: f32) -> T {
        // why use a curve otherwise.
        assert!(self.xs.len() == self.ys.len() && !self.ys.is_empty());

        // First find the x corresponding to this t. (lower bound)
        let mut idx = 0usize;
        for (i, &x) in self.xs.iter().enumerate() {
            if x > t {
                break;
            }

            idx = i;
        }

        let lower_y = unsafe { self.ys.get_unchecked(idx).clone() };
        if idx == self.ys.len() - 1 {
            lower_y
        } else {
            let lower_t: f32 = *unsafe { self.xs.get_unchecked(idx) };
            let higher_t: f32 = *unsafe { self.xs.get_unchecked(idx + 1) };
            let higher_y = unsafe { self.ys.get_unchecked(idx + 1).clone() };

            let slope = (higher_y - lower_y.clone()) * (1.0 / (higher_t - lower_t));
            let val = lower_y.clone() + slope * (t - lower_t);

            val
        }
    }
}

impl CurveNode for Vector2f {}
impl CurveNode for f32 {}
