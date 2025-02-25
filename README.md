# Calculate Generalized (Becker) IRR

## Install

1. Clone the repository
2. Install `maturin`: `pipx install maturin`
3. Build from source: `maturin build`


## Usage

```python
from becker_irr_rs import becker_irr

cfs = [50, -200, 20, 40, 200, 100, -70, -100, 20, 100]
int_disc = 0.07
irr_guess = 0.1
precision = 6

solved_becker_irr = becker_irr(cfs, int_disc, irr_guess, precision)

```

## Reference

Python/Rust implementation of this [SOA article](https://www.soa.org/globalassets/assets/library/newsletters/compact/2008/april/com-2008-iss27-rozar.pdf)
