use crate::digit;
use alloc::string::String;
use core::fmt::Write;
use core::fmt::{Binary, Debug, Display, Formatter, LowerExp, LowerHex, Octal, UpperExp, UpperHex};

macro_rules! fmt {
    ($BUint: ident, $BInt: ident, $Digit: ident) => {
        macro_rules! fmt_method {
            ($format: expr, $format_pad: expr, $pad: expr, $prefix: expr) => {
                #[inline]
                fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
                    let mut format_string = String::new();
                    for digit in self.digits.iter().rev() {
                        if format_string.is_empty() {
                            if digit != &0 {
                                write!(format_string, $format, digit)?;
                            }
                        } else {
                            write!(format_string, $format_pad, digit, $pad)?;
                        }
                    }
                    f.pad_integral(
                        true,
                        $prefix,
                        if format_string.is_empty() {
                            "0"
                        } else {
                            &format_string
                        },
                    )
                }
            };
        }

        impl<const N: usize> Binary for $BUint<N> {
            /*#[inline]
            fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
                f.pad_integral(true, "0b", &self.to_str_radix(2))
            }*/
            fmt_method!("{:b}", "{:01$b}", digit::$Digit::BITS as usize, "0b");
        }

        impl<const N: usize> Debug for $BUint<N> {
            #[inline]
            fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
                Display::fmt(&self, f)
            }
        }

        impl<const N: usize> Display for $BUint<N> {
            /*#[inline]
            fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
                if self.is_zero() {
                    return f.pad_integral(true, "", "0");
                }
                fn digit_to_byte(d: $Digit) -> u8 {
                    match d {
                        0 => 0 + 48,
                        1 => 1 + 48,
                        2 => 2 + 48,
                        3 => 3 + 48,
                        4 => 4 + 48,
                        5 => 5 + 48,
                        6 => 6 + 48,
                        7 => 7 + 48,
                        8 => 8 + 48,
                        9 => 9 + 48,
                        _ => unreachable!(),
                    }
                }
                let mut v: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
                let mut u = *self;
                while !u.is_zero() {
                    let (q, r) = u.div_rem_digit(10);
                    v.push(digit_to_byte(r));
                    u = q;
                }
                v.reverse();
                let s = unsafe { String::from_utf8_unchecked(v) };
                //s.push(digit_to_char(u.digits[0]));
                f.pad_integral(true, "", &s)
            }*/
            #[inline]
            fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
                f.pad_integral(true, "", &self.to_str_radix(10))
            }
        }

        macro_rules! exp_fmt {
            ($e: expr) => {
                #[inline]
                fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
                    let decimal_str = self.to_str_radix(10);
                    let buf = if decimal_str == "0" {
                        format!("{}{}0", 0, $e)
                    } else {
                        let exp = decimal_str.len() - 1;
                        let decimal_str = decimal_str.trim_end_matches('0');
                        if decimal_str.len() == 1 {
                            format!("{}{}{}", &decimal_str[0..1], $e, exp)
                        } else {
                            format!(
                                "{}.{}{}{}",
                                &decimal_str[0..1],
                                &decimal_str[1..],
                                $e,
                                exp
                            )
                        }
                    };
                    f.pad_integral(true, "", &buf)
                }
            };
        }

        impl<const N: usize> LowerExp for $BUint<N> {
            exp_fmt!("e");
        }

        impl<const N: usize> LowerHex for $BUint<N> {
            fmt_method!("{:x}", "{:01$x}", digit::$Digit::HEX_PADDING, "0x");
        }

        impl<const N: usize> Octal for $BUint<N> {
            #[inline]
            fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
                let string = self.to_str_radix(8);
                f.pad_integral(true, "0o", &string)
            }
        }

        impl<const N: usize> UpperExp for $BUint<N> {
            exp_fmt!("E");
        }

        impl<const N: usize> UpperHex for $BUint<N> {
            fmt_method!("{:X}", "{:01$X}", digit::$Digit::HEX_PADDING, "0x");
        }

    };
}

crate::macro_impl!(fmt);
