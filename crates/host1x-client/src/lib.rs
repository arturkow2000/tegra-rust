#![no_std]

use arbitrary_int::u10;

/// A Host1x client: a hardware block that receives commands through Host1x.
///
/// # Safety
/// `CLASS` must be the class ID of the hardware block whose registers are implemented
/// for `Self`. Commands written to registers of `Self` are sent to that class.
pub unsafe trait Client: 'static {
    const CLASS: u10;
}
