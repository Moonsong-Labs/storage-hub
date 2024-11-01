# SHP Treasury Funding

The `shp-treasury-funding` crate provides mechanisms for calculating the percentage amount of tokens from a baseline that will be transferred to the treasury from the charged payment streams. It includes various strategies for calculating the treasury cut based on different configurations.

## Overview

This crate is designed to handle the treasury funding logic for a Substrate-based blockchain. It includes:

- Mechanisms for calculating treasury cuts with different strategies.
- Configurable parameters for fine-tuning the treasury cut calculations.
- Support for fixed percentage cuts, no cuts, and dynamic cuts based on system utilisation.

## Installation

To include this crate in your project, add the following to your `Cargo.toml`:

```toml
[dependencies]
shp-treasury-funding = "0.1.0"
```

## Usage

Here is a basic example of how to use the `shp-treasury-funding` crate:

```rust
use shp_treasury_funding::{FixedCutTreasuryCutCalculator, NoCutTreasuryCutCalculator, TreasuryCutCalculator};
use sp_arithmetic::Perquintill;

fn main() {
    // Example usage of NoCutTreasuryCutCalculator
    let no_cut_calculator = NoCutTreasuryCutCalculator::<u128, u64>(PhantomData);
    let no_cut = no_cut_calculator.calculate_treasury_cut(100, 50, 1000);
    println!("No Cut Treasury Cut: {}", no_cut);

    // Example usage of FixedCutTreasuryCutCalculator
    struct TreasuryCut;
    impl Get<Perquintill> for TreasuryCut {
        fn get() -> Perquintill {
            Perquintill::from_rational(5, 100)
        }
    }
    struct Config;
    impl FixedCutTreasuryCutCalculatorConfig<Perquintill> for Config {
        type Balance = u128;
        type ProvidedUnit = u64;
        type TreasuryCut = TreasuryCut;
    }
    let fixed_cut_calculator = FixedCutTreasuryCutCalculator::<Config, Perquintill>(PhantomData);
    let fixed_cut = fixed_cut_calculator.calculate_treasury_cut(100, 50, 1000);
    println!("Fixed Cut Treasury Cut: {}", fixed_cut);
}
```

## Implementations

The `shp-treasury-funding` crate already has the following implementations:

### NoCutTreasuryCutCalculator

A struct that implements the `TreasuryCutCalculator` trait, where the cut is 0%.

### FixedCutTreasuryCutCalculator

A struct that implements the `TreasuryCutCalculator` trait, where the cut is a fixed percentage.

### LinearThenPowerOfTwoTreasuryCutCalculator

A struct that implements the `TreasuryCutCalculator` trait, where the cut is determined by the `compute_adjustment_over_minimum_cut` function.

### Configuration Traits

- `FixedCutTreasuryCutCalculatorConfig`: Configuration trait for `FixedCutTreasuryCutCalculator`.
- `LinearThenPowerOfTwoTreasuryCutCalculatorConfig`: Configuration trait for `LinearThenPowerOfTwoTreasuryCutCalculator`.

## Equations

The crate uses several equations to manage the calculation of treasury cuts. Below are the key equations used:

### Equation 1: Fixed Cut Treasury Calculation

$$\text{treasury\_cut} = \text{amount\_to\_charge} \times \text{TreasuryCut}$$

Where:

- `treasury_cut` is the amount to be transferred to the treasury.
- `amount_to_charge` is the total amount to be charged.
- `TreasuryCut` is the fixed percentage cut for the treasury.

### Equation 2: Dynamic Treasury Cut Calculation

$$\text{treasury\_cut} = \text{minimum\_cut} + (\text{maximum\_cut} - \text{minimum\_cut}) \times \text{adjustment}$$

Where:

- `treasury_cut` is the percentage amount to be transferred to the treasury.
- `minimum_cut` is the minimum percentage cut for the treasury.
- `maximum_cut` is the maximum percentage cut for the treasury.
- `adjustment` is calculated using the following formula:

$$\text{adjustment}(x) = \begin{cases}
1 - \frac{x}{x_{\text{ideal}}} & \text{for } 0 \leq x \leq x_{\text{ideal}} \\
1 - 2^{\frac{x_{\text{ideal}} - x}{d}} & \text{for } x_{\text{ideal}} < x \leq 1 
\end{cases}$$

Where:

- `x` is the system utilisation rate.
- `x_ideal` is the ideal system utilisation rate.
- `d` is the falloff or decay rate.

The falloff or decay rate `d` is used to determine how quickly the treasury cut increases as the system utilisation rate moves away from the ideal rate into higher rates.
A lower `d` value will result in a steeper increase in the treasury cut as the system utilisation rate grows, while a higher `d` value will result in a more gradual increase.
Caution should be taken when setting the `d` value to ensure that the treasury cut does not increase too rapidly or too slowly. A hard cap at $d = 0.01$ is implemented, 
disallowing the treasury cut from increasing too rapidly, and it's advised to keep the `d` value below $\frac{1-x_{ideal}}{3}$ as for values higher than this the maximum treasury cut
obtainable (at 100% utilisation) does not reach more than 90% of the configured maximum cut.

Below you can find a GIF showing how the adjustment changes based on the decay rate, with the ideal rate set at 85% and the decay rate ranging from 0.01 to 0.05:

<img src="resources/Decay rate changes.gif" alt="Changes in adjustment based on the decay rate" style="width:40%; height:auto;">

## Graphs

Below are the graphs representing the equations used in the crate, where the `x` axis represents the system utilisation rate and the `y` axis represents the treasury cut percentage.

### Fixed Cut Treasury Calculation Graph

The graph below shows the treasury cut calculation for a fixed percentage cut of 5%.

<img src="resources/Fixed treasury cut graph.png" alt="Fixed Cut Treasury Calculation Graph" style="width:40%; height:auto;">

### Dynamic Treasury Cut Calculation Graph

The graph below shows the treasury cut calculation for a dynamic cut based on system utilisation rate, with the ideal rate set at 85%, the decay rate set at 0.02,
the minimum cut percentage to the treasury set at 1%, and the maximum cut percentage set at 5%.

<img src="resources/Dynamic treasury cut graph.png" alt="Dynamic Treasury Cut Calculation Graph" style="width:40%; height:auto;">
