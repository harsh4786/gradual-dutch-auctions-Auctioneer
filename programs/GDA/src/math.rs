
use anchor_lang::prelude::*;
#[derive(Debug, Copy,Clone)]
pub struct Decimal {
    pub val: u128,
    pub scale: u8,
}
impl Decimal {
    pub fn new(value: u128, scale: u8) -> Self {
        Self { val: value, scale }
    }
    pub fn denominator(self) -> u128 {
        10u128.pow(self.scale.into())
    }
    pub fn euler_value() -> Self{
        Self{
            val: 2718281828459045,
            scale: 15,
        }
    }
    pub fn from_integer(integer: u64) -> Self {
        Decimal {
            val: integer.into(),
            scale: 0,
        }
    }
    
    pub fn to_u64(self) -> u64 {
        self.val.try_into().unwrap()
    }
    
    pub fn to_scale(self, scale: u8) -> Self {
        Self {
            val: if self.scale > scale {
                self.val
                    .checked_div(10u128.pow((self.scale - scale).into()))
                    .unwrap()
            } else {
                self.val
                    .checked_mul(10u128.pow((scale - self.scale).into()))
                    .unwrap()
            },
            scale,
        }
    }
    pub fn to_scale_up(self, scale: u8) -> Self {
        let decimal = Self::new(self.val, scale);
        if self.scale >= scale {
            decimal.div_up(Self::new(
                10u128.pow((self.scale - scale).try_into().unwrap()),
                0,
            ))
        } else {
            decimal.mul_up(Self::new(
                10u128.pow((scale - self.scale).try_into().unwrap()),
                0,
            ))
        }
    }
}

impl Mul<Decimal> for Decimal {
    fn mul(self, value: Decimal) -> Self {
        Self {
            val: self
                .val
                .checked_mul(value.val)
                .unwrap()
                .checked_div(value.denominator())
                .unwrap(),
            scale: self.scale,
        }
    }
}
impl Mul<u128> for Decimal {
    fn mul(self, value: u128) -> Self {
        Self {
            val: self.val.checked_mul(value).unwrap(),
            scale: self.scale,
        }
    }
}
impl MulUp<Decimal> for Decimal {
    fn mul_up(self, other: Decimal) -> Self {
        let denominator = other.denominator();

        Self {
            val: self
                .val
                .checked_mul(other.val)
                .unwrap()
                .checked_add(denominator.checked_sub(1).unwrap())
                .unwrap()
                .checked_div(denominator)
                .unwrap(),
            scale: self.scale,
        }
    }
}
impl Add<Decimal> for Decimal {
    fn add(self, value: Decimal) -> Result<Self> {
        require!(self.scale == value.scale, MyError::DifferentScale);

        Ok(Self {
            val: self.val.checked_add(value.val).unwrap(),
            scale: self.scale,
        })
    }
}
impl Sub<Decimal> for Decimal {
    fn sub(self, value: Decimal) -> Result<Self> {
        require!(self.scale == value.scale, MyError::DifferentScale);
        Ok(Self {
            val: self.val.checked_sub(value.val).unwrap(),
            scale: self.scale,
        })
    }
}
impl Div<Decimal> for Decimal {
    fn div(self, other: Decimal) -> Self {
        Self {
            val: self
                .val
                .checked_mul(other.denominator())
                .unwrap()
                .checked_div(other.val)
                .unwrap(),
            scale: self.scale,
        }
    }
}
impl DivUp<Decimal> for Decimal {
    fn div_up(self, other: Decimal) -> Self {
        Self {
            val: self
                .val
                .checked_mul(other.denominator())
                .unwrap()
                .checked_add(other.val.checked_sub(1).unwrap())
                .unwrap()
                .checked_div(other.val)
                .unwrap(),
            scale: self.scale,
        }
    }
}
impl DivScale<Decimal> for Decimal {
    fn div_to_scale(self, other: Decimal, to_scale: u8) -> Self {
        let decimal_difference = (self.scale as i32)
            .checked_sub(to_scale.into())
            .unwrap()
            .checked_sub(other.scale.into())
            .unwrap();

        let val = if decimal_difference > 0 {
            self.val
                .checked_div(other.val)
                .unwrap()
                .checked_div(10u128.pow(decimal_difference.try_into().unwrap()))
                .unwrap()
        } else {
            self.val
                .checked_mul(10u128.pow((-decimal_difference).try_into().unwrap()))
                .unwrap()
                .checked_div(other.val)
                .unwrap()
        };
        Self {
            val,
            scale: to_scale,
        }
    }
}
impl PowAccuracy<u128> for Decimal {
    fn pow_with_accuracy(self, exp: u128) -> Self {
        let one = Decimal {
            val: self.denominator(),
            scale: self.scale,
        };
        if exp == 0 {
            return one;
        }
        let mut current_exp = exp;
        let mut base = self;
        let mut result = one;

        while current_exp > 0 {
            if current_exp % 2 != 0 {
                result = result.mul(base);
            }
            current_exp /= 2;
            base = base.mul(base);
        }
        return result;
    }
}
impl Into<u64> for Decimal {
    fn into(self) -> u64 {
        self.val.try_into().unwrap()
    }
}
impl Into<u128> for Decimal {
    fn into(self) -> u128 {
        self.val.try_into().unwrap()
    }
}


pub trait Sub<T>: Sized {
    fn sub(self, rhs: T) -> Result<Self>;
}
pub trait Add<T>: Sized {
    fn add(self, rhs: T) -> Result<Self>;
}
pub trait Div<T>: Sized {
    fn div(self, rhs: T) -> Self;
}
pub trait DivScale<T> {
    fn div_to_scale(self, rhs: T, to_scale: u8) -> Self;
}
pub trait DivUp<T>: Sized {
    fn div_up(self, rhs: T) -> Self;
}
pub trait Mul<T>: Sized {
    fn mul(self, rhs: T) -> Self;
}
pub trait MulUp<T>: Sized {
    fn mul_up(self, rhs: T) -> Self;
}
pub trait PowAccuracy<T>: Sized {
    fn pow_with_accuracy(self, rhs: T) -> Self;
}

#[error_code]
pub enum MyError{
    #[msg("Invalid")]
    DifferentScale,
}


