use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use std::fmt;

/// Custom error type for Becker IRR calculations
#[derive(Debug)]
pub enum BeckerError {
    MaxIterationsReached(&'static str),
    EmptyEarnings,
    InvalidInput(&'static str),
}

impl fmt::Display for BeckerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BeckerError::MaxIterationsReached(msg) => write!(f, "Max iterations reached: {}", msg),
            BeckerError::EmptyEarnings => write!(f, "Empty earnings sequence"),
            BeckerError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

/// Convert our Rust error to a Python error
impl From<BeckerError> for PyErr {
    fn from(err: BeckerError) -> PyErr {
        PyValueError::new_err(err.to_string())
    }
}

/// Result type for Becker IRR calculations
pub type BeckerResult = Result<f64, BeckerError>;

/// Configuration for the IRR calculation algorithm
#[derive(Debug, Clone, Copy)]
pub struct IrrConfig {
    max_iterations: u32,
    init_increment: f64,
    tolerance: f64,
}

impl Default for IrrConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            init_increment: 0.05,
            tolerance: 1e-10,
        }
    }
}

/// Calculate the Becker OBt value for a series of earnings
#[inline]
fn becker_obt_rs(earnings: &[f64], disc_rate: f64, becker_irr: f64) -> f64 {
    if earnings.is_empty() {
        return 0.0;
    }

    // Start with first earning for efficiency
    let mut obt = earnings[0];

    // Pre-calculate growth factors
    let pos_factor = 1.0 + disc_rate;
    let neg_factor = 1.0 + becker_irr;

    // Process remaining earnings
    earnings
        .iter()
        .skip(1) // Skip first element as we already used it
        .for_each(|&earning| {
            obt = obt * if obt < 0.0 { neg_factor } else { pos_factor } + earning;
        });

    obt
}

/// Python wrapper for becker_obt
#[pyfunction]
fn becker_obt(earnings: Vec<f64>, disc_rate: f64, becker_irr: f64) -> f64 {
    becker_obt_rs(&earnings, disc_rate, becker_irr)
}

/// Find initial bounds for the IRR calculation
fn find_bounds(
    earnings: &[f64],
    int_disc: f64,
    initial_guess: f64,
    config: &IrrConfig,
) -> BeckerResult {
    let mut obt = becker_obt_rs(earnings, int_disc, initial_guess);

    // Early exit if initial guess is very close to solution
    if obt.abs() < config.tolerance {
        return Ok(initial_guess);
    }

    if obt < 0.0 {
        let mut irr_b = initial_guess;
        let mut step = config.init_increment;

        // Binary search for bound
        for _ in 0..config.max_iterations {
            irr_b -= step;
            let new_obt = becker_obt_rs(earnings, int_disc, irr_b);

            if new_obt >= 0.0 {
                return Ok(irr_b);
            }

            if new_obt < obt {
                step *= 2.0; // Increase step size if going wrong direction
            }
            obt = new_obt;
        }
    } else {
        let mut irr_a = initial_guess;
        let mut step = config.init_increment;

        // Binary search for bound
        for _ in 0..config.max_iterations {
            irr_a += step;
            let new_obt = becker_obt_rs(earnings, int_disc, irr_a);

            if new_obt <= 0.0 {
                return Ok(irr_a);
            }

            if new_obt > obt {
                step *= 2.0; // Increase step size if going wrong direction
            }
            obt = new_obt;
        }
    }

    Err(BeckerError::MaxIterationsReached(
        "Could not find initial bounds",
    ))
}

/// Core implementation of becker_irr
fn internal_becker_irr(
    earnings: &[f64],
    int_disc: f64,
    irr_guess: f64,
    decimals: i32,
) -> BeckerResult {
    // Input validation
    if earnings.is_empty() {
        return Err(BeckerError::EmptyEarnings);
    }

    if int_disc < -1.0 || decimals < 0 {
        return Err(BeckerError::InvalidInput(
            "Invalid discount rate or decimals",
        ));
    }

    // Handle simple cases
    match earnings.len() {
        1 => {
            return Ok(if earnings[0] == 0.0 {
                0.0
            } else if earnings[0] > 0.0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            })
        }
        _ if earnings.iter().all(|&x| x == 0.0) => return Ok(0.0),
        _ => {}
    }

    let config = IrrConfig {
        tolerance: 10.0f64.powi(-decimals),
        ..IrrConfig::default()
    };

    // Find initial bounds
    let bound = find_bounds(earnings, int_disc, irr_guess, &config)?;
    let (mut irr_a, mut irr_b) = if becker_obt_rs(earnings, int_disc, irr_guess) < 0.0 {
        (irr_guess, bound)
    } else {
        (bound, irr_guess)
    };

    // Early exit if bounds are already close enough
    if (irr_a - irr_b).abs() < config.tolerance {
        return Ok((irr_a + irr_b) / 2.0);
    }

    // Binary search with adaptive precision
    for _ in 0..config.max_iterations {
        let irr_mid = (irr_a + irr_b) / 2.0;

        // Early exit if we've reached desired precision
        if (irr_a - irr_b).abs() <= config.tolerance {
            return Ok(irr_mid);
        }

        let obt = becker_obt_rs(earnings, int_disc, irr_mid);

        // Early exit if we found an exact solution
        if obt.abs() < config.tolerance {
            return Ok(irr_mid);
        }

        if obt < 0.0 {
            irr_a = irr_mid;
        } else {
            irr_b = irr_mid;
        }
    }

    Err(BeckerError::MaxIterationsReached(
        "Binary search did not converge",
    ))
}

/// Python wrapper for becker_irr
#[pyfunction]
fn becker_irr(earnings: Vec<f64>, int_disc: f64, irr_guess: f64, decimals: i32) -> PyResult<f64> {
    match internal_becker_irr(&earnings, int_disc, irr_guess, decimals) {
        Ok(result) => Ok(result),
        Err(err) => Err(err.into()),
    }
}

/// Define the Python module
#[pymodule]
fn becker_irr_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(becker_irr, m)?)?;
    m.add_function(wrap_pyfunction!(becker_obt, m)?)?;

    // Add module docstring
    m.add("__doc__", "Rust implementation of Becker IRR calculation")?;

    Ok(())
}
