use crate::*;

#[error_code]
#[derive(Eq, PartialEq)]
pub enum RandomnessRequestError {
    #[msg("FunctionAccount was not validated successfully")]
    FunctionValidationFailed,
    #[msg("FunctionRequestAccount status should be 'RequestSuccess'")]
    SwitchboardRequestNotSuccessful,
    #[msg("Round is inactive")]
    RoundInactive,
}
