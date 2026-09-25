use crate::constants::{MINING_PERIOD_SECONDS, TOTAL_POWER, TOTAL_SUPPLY_BASE_UNITS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardMathError {
    Overflow,
    DivisionByZero,
}

pub fn calculate_reward(
    elapsed_seconds: u64,
    class_multiplier: u64,
) -> Result<u64, RewardMathError> {
    let numerator = u128::from(elapsed_seconds)
        .checked_mul(u128::from(class_multiplier))
        .ok_or(RewardMathError::Overflow)?
        .checked_mul(u128::from(TOTAL_SUPPLY_BASE_UNITS))
        .ok_or(RewardMathError::Overflow)?;

    let denominator = u128::from(MINING_PERIOD_SECONDS)
        .checked_mul(u128::from(TOTAL_POWER))
        .ok_or(RewardMathError::Overflow)?;

    if denominator == 0 {
        return Err(RewardMathError::DivisionByZero);
    }

    let reward = numerator
        .checked_div(denominator)
        .ok_or(RewardMathError::DivisionByZero)?;

    reward.try_into().map_err(|_| RewardMathError::Overflow)
}

pub fn cap_reward_to_remaining_supply(calculated_reward: u64, remaining_supply: u64) -> u64 {
    calculated_reward.min(remaining_supply)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        CLASS_TABLE, MINING_PERIOD_SECONDS, TOTAL_PASS_COUNT, TOTAL_POWER, TOTAL_SUPPLY_BASE_UNITS,
    };

    #[test]
    fn zero_elapsed_time_mints_zero() {
        assert_eq!(calculate_reward(0, 1).unwrap(), 0);
    }

    #[test]
    fn each_class_multiplier_matches_formula() {
        for class in CLASS_TABLE {
            let expected =
                ((86_400u128) * u128::from(class.multiplier) * u128::from(TOTAL_SUPPLY_BASE_UNITS)
                    / (u128::from(MINING_PERIOD_SECONDS) * u128::from(TOTAL_POWER)))
                    as u64;
            assert_eq!(
                calculate_reward(86_400, class.multiplier).unwrap(),
                expected,
                "class {}",
                class.name
            );
        }
    }

    #[test]
    fn multi_year_elapsed_time_is_supported() {
        let reward = calculate_reward(5 * 365 * 24 * 60 * 60, 64).unwrap();
        assert!(reward > 0);
    }

    #[test]
    fn overflow_edge_case_returns_error() {
        let result = calculate_reward(u64::MAX, 64);
        assert_eq!(result, Err(RewardMathError::Overflow));
    }

    #[test]
    fn aggregate_twenty_year_emission_stays_within_rounding_distance() {
        let mut aggregate: u128 = 0;
        for class in CLASS_TABLE {
            let reward = calculate_reward(MINING_PERIOD_SECONDS, class.multiplier).unwrap() as u128;
            aggregate += reward * u128::from(class.count);
        }

        let total_supply = u128::from(TOTAL_SUPPLY_BASE_UNITS);
        assert!(aggregate <= total_supply);
        let rounding_gap = total_supply - aggregate;
        assert!(rounding_gap <= u128::from(TOTAL_PASS_COUNT));
    }

    #[test]
    fn cap_never_exceeds_remaining_supply() {
        assert_eq!(cap_reward_to_remaining_supply(500, 200), 200);
        assert_eq!(cap_reward_to_remaining_supply(100, 200), 100);
    }
}
