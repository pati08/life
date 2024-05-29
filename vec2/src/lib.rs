use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Hash, Serialize, Deserialize)]
pub struct Vector2<T> {
    /// X component of the vector.
    pub x: T,

    /// Y component of the vector.
    pub y: T,
}

impl<T: Eq> Eq for Vector2<T> {}

impl Vector2<f64> {
    /// Shorthand for writing `Vector2::new(0.0, -1.0)`.
    pub const DOWN: Self = Self { x: 0.0, y: -1.0 };

    /// Shorthand for writing `Vector2::new(-1.0, 0.0)`.
    pub const LEFT: Self = Self { x: -1.0, y: 0.0 };

    /// Shorthand for writing `Vector2::new(f64::NEG_INFINITY, f64::NEG_INFINITY)`.
    pub const NEGATIVE_INFINITY: Self = Self {
        x: f64::NEG_INFINITY,
        y: f64::NEG_INFINITY,
    };

    /// Shorthand for writing `Vector2::new(1.0, 1.0)`.
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };

    /// Shorthand for writing `Vector2::new(f64::INFINITY, f64::INFINITY)`.
    pub const POSITIVE_INFINITY: Self = Self {
        x: f64::INFINITY,
        y: f64::INFINITY,
    };

    /// Shorthand for writing `Vector2::new(1.0, 0.0)`.
    pub const RIGHT: Self = Self { x: 1.0, y: 0.0 };

    /// Shorthand for writing `Vector2::new(0.0, 1.0)`.
    pub const UP: Self = Self { x: 0.0, y: 1.0 };

    /// Shorthand for writing `Vector2::new(0.0, 0.0)`.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Returns the length of this vector.
    pub fn magnitude(&self) -> f64 {
        f64::sqrt((self.x * self.x) + (self.y * self.y))
    }

    /// Returns the squared length of this vector.
    pub fn sqr_magnitude(&self) -> f64 {
        (self.x * self.x) + (self.y * self.y)
    }

    /// Returns this vector with a magnitude of 1.
    pub fn normalized(&self) -> Self {
        let mut v = Self::new(self.x, self.y);
        v.normalize();
        v
    }

    /// Makes this vector have a magnitude of 1.
    pub fn normalize(&mut self) {
        let magnitude = self.magnitude();
        if magnitude > Self::K_EPSILON {
            *self /= magnitude;
        } else {
            *self = Self::ZERO;
        }
    }

    /// Gets the unsigned angle in degrees between from and to.
    pub fn angle(from: Self, to: Self) -> f64 {
        let denominator = f64::sqrt(from.sqr_magnitude() * to.sqr_magnitude());
        if denominator < Self::K_EPSILON_NORMAL_SQRT {
            0.0
        } else {
            let dot = f64::clamp(Self::dot(from, to), -1.0, 1.0);
            f64::to_degrees(f64::acos(dot))
        }
    }

    /// Returns a copy of vector with its magnitude clamped to max_length.
    pub fn clamp_magnitude(vector: Self, max_length: f64) -> Self {
        let sqr_magnitude = vector.sqr_magnitude();
        if sqr_magnitude > max_length * max_length {
            let mag = f64::sqrt(sqr_magnitude);

            let normalized_x = vector.x / mag;
            let normalized_y = vector.y / mag;
            Self::new(normalized_x * max_length, normalized_y * max_length)
        } else {
            vector
        }
    }

    /// Returns the distance between a and b.
    pub fn distance(a: Self, b: Self) -> f64 {
        let diff_x = a.x - b.x;
        let diff_y = a.y - b.y;
        f64::sqrt((diff_x * diff_x) + (diff_y * diff_y))
    }

    /// Dot product of two vectors.
    pub fn dot(lhs: Self, rhs: Self) -> f64 {
        (lhs.x * rhs.x) + (lhs.y * rhs.y)
    }

    /// Linearly interpolates between vectors a and b by t.
    pub fn lerp(a: Self, b: Self, mut t: f64) -> Self {
        t = f64::clamp(t, 0.0, 1.0);
        Self::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    }

    /// Linearly interpolates between vectors a and b by t.
    pub fn lerp_unclamped(a: Self, b: Self, t: f64) -> Self {
        Self::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    }

    /// Returns a vector that is made from the largest components of two vectors.
    pub fn max(lhs: Self, rhs: Self) -> Self {
        Self::new(f64::max(lhs.x, rhs.x), f64::max(lhs.y, rhs.y))
    }

    /// Returns a vector that is made from the smallest components of two vectors.
    pub fn min(lhs: Self, rhs: Self) -> Self {
        Self::new(f64::min(lhs.x, rhs.x), f64::min(lhs.y, rhs.y))
    }

    /// Moves a point current towards target.
    pub fn move_towards(current: Self, target: Self, max_distance_delta: f64) -> Self {
        let to_vector_x = target.x - current.x;
        let to_vector_y = target.y - current.y;

        let sq_dist = (to_vector_x * to_vector_x) + (to_vector_y * to_vector_y);

        if sq_dist == 0.0
            || (max_distance_delta >= 0.0 && sq_dist <= max_distance_delta * max_distance_delta)
        {
            target
        } else {
            let dist = f64::sqrt(sq_dist);

            Self::new(
                current.x + ((to_vector_x / dist) * max_distance_delta),
                current.y + ((to_vector_y / dist) * max_distance_delta),
            )
        }
    }

    /// Returns the 2D vector perpendicular to this 2D vector. The result is always rotated 90-degrees in a counter-clockwise direction for a 2D coordinate system where the positive Y axis goes up.
    pub fn perpendicular(in_direction: Self) -> Self {
        Self::new(-in_direction.y, in_direction.x)
    }

    /// Reflects a vector off the vector defined by a normal.
    pub fn reflect(in_direction: Self, in_normal: Self) -> Self {
        let factor = -2.0 * Self::dot(in_normal, in_direction);
        Self::new(
            (factor * in_normal.x) + in_direction.x,
            (factor * in_normal.y) + in_direction.y,
        )
    }

    /// Multiplies two vectors component-wise.
    pub fn scale(a: Self, b: Self) -> Self {
        Self::new(a.x * b.x, a.y * b.y)
    }

    /// Gets the signed angle in degrees between from and to.
    pub fn signed_angle(from: Self, to: Self) -> f64 {
        let unsigned_angle = Self::angle(from, to);
        let sign = f64::signum((from.x * to.y) - (from.y * to.x));
        unsigned_angle * sign
    }

    const K_EPSILON: f64 = 0.00001;
    const K_EPSILON_NORMAL_SQRT: f64 = 1e-15;
}

impl<T> Vector2<T> {
    /// Constructs a new vector with given x, y components.
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
    /// Set x and y components of an existing Vector2.
    pub fn set(&mut self, new_x: T, new_y: T) {
        self.x = new_x;
        self.y = new_y;
    }
}

impl<T: Copy> From<[T; 2]> for Vector2<T> {
    fn from(value: [T; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
        }
    }
}

impl<T> From<Vector2<T>> for [T; 2] {
    fn from(value: Vector2<T>) -> [T; 2] {
        [value.x, value.y]
    }
}

impl<T> From<(T, T)> for Vector2<T> {
    fn from(value: (T, T)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

impl<T> From<Vector2<T>> for (T, T) {
    fn from(value: Vector2<T>) -> (T, T) {
        (value.x, value.y)
    }
}

impl<T: Sub<Output = T> + Copy> Sub for Vector2<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl<T: Sub<Output = T> + Copy> SubAssign for Vector2<T> {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Vector2<T> {
    type Output = Self;

    fn mul(self, other: T) -> Self {
        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

// Unfortunately, rust's trait implementation rules prohibit this.
//
// impl<T> Mul<Vector2<T>> for T
// where
//     T: Copy,
//     Vector2<T>: Mul<T, Output = Vector2<T>>,
// {
//     type Output = Vector2<T>;
//
//     fn mul(self, other: Vector2<T>) -> Vector2<T> {
//         other * self
//     }
// }

impl<T: Mul<Output = T> + Copy> MulAssign<T> for Vector2<T> {
    fn mul_assign(&mut self, other: T) {
        *self = Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl<T: Div<Output = T> + Copy> Div<T> for Vector2<T> {
    type Output = Self;

    fn div(self, other: T) -> Self {
        Self {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl<T: Div<Output = T> + Copy> DivAssign<T> for Vector2<T> {
    fn div_assign(&mut self, other: T) {
        *self = Self {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl<T: Add<Output = T> + Copy> Add for Vector2<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl<T: Add<Output = T> + Copy> AddAssign for Vector2<T> {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl<T: fmt::Display> fmt::Display for Vector2<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl<T: Neg<Output = T>> Neg for Vector2<T> {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Vector2<f32> {
    /// Shorthand for writing `Vector2::new(0.0, -1.0)`.
    pub const DOWN: Self = Self { x: 0.0, y: -1.0 };

    /// Shorthand for writing `Vector2::new(-1.0, 0.0)`.
    pub const LEFT: Self = Self { x: -1.0, y: 0.0 };

    /// Shorthand for writing `Vector2::new(f32::NEG_INFINITY, f32::NEG_INFINITY)`.
    pub const NEGATIVE_INFINITY: Self = Self {
        x: f32::NEG_INFINITY,
        y: f32::NEG_INFINITY,
    };

    /// Shorthand for writing `Vector2::new(1.0, 1.0)`.
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };

    /// Shorthand for writing `Vector2::new(f32::INFINITY, f32::INFINITY)`.
    pub const POSITIVE_INFINITY: Self = Self {
        x: f32::INFINITY,
        y: f32::INFINITY,
    };

    /// Shorthand for writing `Vector2::new(1.0, 0.0)`.
    pub const RIGHT: Self = Self { x: 1.0, y: 0.0 };

    /// Shorthand for writing `Vector2::new(0.0, 1.0)`.
    pub const UP: Self = Self { x: 0.0, y: 1.0 };

    /// Shorthand for writing `Vector2::new(0.0, 0.0)`.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Returns the length of this vector.
    pub fn magnitude(&self) -> f32 {
        f32::sqrt((self.x * self.x) + (self.y * self.y))
    }

    /// Returns the squared length of this vector.
    pub fn sqr_magnitude(&self) -> f32 {
        (self.x * self.x) + (self.y * self.y)
    }

    /// Returns this vector with a magnitude of 1.
    pub fn normalized(&self) -> Self {
        let mut v = Self::new(self.x, self.y);
        v.normalize();
        v
    }

    /// Makes this vector have a magnitude of 1.
    pub fn normalize(&mut self) {
        let magnitude = self.magnitude();
        if magnitude > Self::K_EPSILON {
            *self /= magnitude;
        } else {
            *self = Self::ZERO;
        }
    }

    /// Gets the unsigned angle in degrees between from and to.
    pub fn angle(from: Self, to: Self) -> f32 {
        let denominator = f32::sqrt(from.sqr_magnitude() * to.sqr_magnitude());
        if denominator < Self::K_EPSILON_NORMAL_SQRT {
            0.0
        } else {
            let dot = f32::clamp(Self::dot(from, to), -1.0, 1.0);
            f32::to_degrees(f32::acos(dot))
        }
    }

    /// Returns a copy of vector with its magnitude clamped to max_length.
    pub fn clamp_magnitude(vector: Self, max_length: f32) -> Self {
        let sqr_magnitude = vector.sqr_magnitude();
        if sqr_magnitude > max_length * max_length {
            let mag = f32::sqrt(sqr_magnitude);

            let normalized_x = vector.x / mag;
            let normalized_y = vector.y / mag;
            Self::new(normalized_x * max_length, normalized_y * max_length)
        } else {
            vector
        }
    }

    /// Returns the distance between a and b.
    pub fn distance(a: Self, b: Self) -> f32 {
        let diff_x = a.x - b.x;
        let diff_y = a.y - b.y;
        f32::sqrt((diff_x * diff_x) + (diff_y * diff_y))
    }

    /// Dot product of two vectors.
    pub fn dot(lhs: Self, rhs: Self) -> f32 {
        (lhs.x * rhs.x) + (lhs.y * rhs.y)
    }

    /// Linearly interpolates between vectors a and b by t.
    pub fn lerp(a: Self, b: Self, mut t: f32) -> Self {
        t = f32::clamp(t, 0.0, 1.0);
        Self::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    }

    /// Linearly interpolates between vectors a and b by t.
    pub fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self {
        Self::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    }

    /// Returns a vector that is made from the largest components of two vectors.
    pub fn max(lhs: Self, rhs: Self) -> Self {
        Self::new(f32::max(lhs.x, rhs.x), f32::max(lhs.y, rhs.y))
    }

    /// Returns a vector that is made from the smallest components of two vectors.
    pub fn min(lhs: Self, rhs: Self) -> Self {
        Self::new(f32::min(lhs.x, rhs.x), f32::min(lhs.y, rhs.y))
    }

    /// Moves a point current towards target.
    pub fn move_towards(current: Self, target: Self, max_distance_delta: f32) -> Self {
        let to_vector_x = target.x - current.x;
        let to_vector_y = target.y - current.y;

        let sq_dist = (to_vector_x * to_vector_x) + (to_vector_y * to_vector_y);

        if sq_dist == 0.0
            || (max_distance_delta >= 0.0 && sq_dist <= max_distance_delta * max_distance_delta)
        {
            target
        } else {
            let dist = f32::sqrt(sq_dist);

            Self::new(
                current.x + ((to_vector_x / dist) * max_distance_delta),
                current.y + ((to_vector_y / dist) * max_distance_delta),
            )
        }
    }

    /// Returns the 2D vector perpendicular to this 2D vector. The result is always rotated 90-degrees in a counter-clockwise direction for a 2D coordinate system where the positive Y axis goes up.
    pub fn perpendicular(in_direction: Self) -> Self {
        Self::new(-in_direction.y, in_direction.x)
    }

    /// Reflects a vector off the vector defined by a normal.
    pub fn reflect(in_direction: Self, in_normal: Self) -> Self {
        let factor = -2.0 * Self::dot(in_normal, in_direction);
        Self::new(
            (factor * in_normal.x) + in_direction.x,
            (factor * in_normal.y) + in_direction.y,
        )
    }

    /// Multiplies two vectors component-wise.
    pub fn scale(a: Self, b: Self) -> Self {
        Self::new(a.x * b.x, a.y * b.y)
    }

    /// Gets the signed angle in degrees between from and to.
    pub fn signed_angle(from: Self, to: Self) -> f32 {
        let unsigned_angle = Self::angle(from, to);
        let sign = f32::signum((from.x * to.y) - (from.y * to.x));
        unsigned_angle * sign
    }

    const K_EPSILON: f32 = 0.00001;
    const K_EPSILON_NORMAL_SQRT: f32 = 1e-15;
}
