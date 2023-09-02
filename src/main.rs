use halo2_proofs::{
    plonk::{ConstraintSystem, Error, Column, Advice, Fixed, Selector, Instance, Expression},
    arithmetic::Field,
    circuit::{Layouter, Value, SimpleFloorPlanner, AssignedCell},
    poly::Rotation, pasta::{Fp, group::{prime::PrimeCurveAffine, ff::PrimeField}},
};

use std::{marker::PhantomData, os::windows::prelude::IntoRawSocket, ops::Mul};

struct ACell<F: PrimeField> (AssignedCell<F,F>);

struct GachaConfig<F: PrimeField, const MODULUS: u64, const MULTIPLIER: u64, const ADDER: u64> {
    adv: [Column<Advice>; 2],
    divisor: Column<Advice>,
    selector: Selector,
    _marker: PhantomData<F>,
}

impl<F: PrimeField, const MODULUS: u64, const MULTIPLIER: u64, const ADDER: u64> GachaConfig<F, MODULUS, MULTIPLIER, ADDER> {
    fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let adv_0 = meta.advice_column();
        let adv_1 = meta. advice_column();
        let divisor = meta.advice_column();
        let selector = meta.selector();

        meta.enable_equality(adv_0);
        meta.enable_equality(adv_1);

        meta.create_gate("linear congruence", |meta| {
            let x = meta.query_advice(adv_0, Rotation::cur());
            let b = meta.query_advice(adv_1, Rotation::cur());
            let d = meta.query_advice(divisor, Rotation::cur());

            let a = Expression::Constant(F::from(MULTIPLIER));
            let m = Expression::Constant(F::from(MODULUS));
            let c = Expression::Constant(F::from(ADDER));

            let s = meta.query_selector(selector);

            vec![ s * (a * x - b - m * d) ]
        });

        GachaConfig {
            adv: [adv_0, adv_1],
            divisor: divisor, 
            selector: selector, 
            _marker: PhantomData
        }
    }

    fn assign_first_row(
        &self,
        mut layouter: impl Layouter<F>,
        seed: Value<F>,
    ) -> Result<(ACell<F>, ACell<F>, ACell<F>), Error> {
        layouter.assign_region(|| "first row with seed", |mut region| {
            let offset = 0;

            self.selector.enable(&mut region, offset)?;

            let next_val = seed.mul(Value::known(F::from(MULTIPLIER))) + Value::known(F::from(ADDER));

            let first_cell = region.assign_advice(|| "seed", self.adv[0], offset, || seed).map(ACell)?;
            let next_cell = region.assign_advice(|| "next value mod m", self.adv[1], offset, || next_val).map(ACell)?;
            let quot_cell = region.assign_advice(|| "quotient", self.adv[1], offset, || Value::known(F::ZERO)).map(ACell)?;

            Ok((first_cell, next_cell, quot_cell))
        })
    }

    // fn assign_first_row(
    //     &self,
    //     mut layouter: impl Layouter<F>,
    //     seed: u64
    // ) -> Result<(ACell<F>, ACell<F>, ACell<F>), Error> {
    //     layouter.assign_region(|| "first row with seed", |mut region| {
    //         let offset = 0;

    //         self.selector.enable(&mut region, offset)?;

    //         let first_val = Value::known(F::from(seed));
    //         let next = seed * MULTIPLIER + ADDER;
    //         let next_quot = next / MODULUS;
    //         let next_rem = next % MODULUS;
    //         let quot_val = Value::known(F::from(next_quot));
    //         let rem_val = Value::known(F::from(next_rem));

    //         let first_cell = region.assign_advice(|| "seed", self.adv[0], offset, || first_val).map(ACell)?;
    //         let next_cell = region.assign_advice(|| "next value mod m", self.adv[1], offset, || rem_val).map(ACell)?;
    //         let quot_cell = region.assign_advice(|| "quotient", self.adv[1], offset, || quot_val).map(ACell)?;

    //         Ok((first_cell, next_cell, quot_cell))
    //     })
    // }

    fn assign_next_row(
        &self,
        mut layouter: impl Layouter<F>,
        prev: &ACell<F>,
    ) -> Result<(ACell<F>, ACell<F>, ACell<F>), Error> {
        layouter.assign_region(|| "linear operation", |mut region| {
            let offset = 0;

            self.selector.enable(&mut region, offset)?;

            let prev_u32: u32 = prev.0.value().copied().map(|a| {
                a.to_repr()
            });
            let prev_u64: u64 = prev_u32 as u64;
            let first_val = Value::known(F::from(prev_u64));
            let next = prev_u64 * MULTIPLIER + ADDER;
            let next_quot = next / MODULUS;
            let next_rem = next % MODULUS;
            let quot_val = Value::known(F::from(next_quot));
            let rem_val = Value::known(F::from(next_rem));

            let first_cell = region.assign_advice(|| "seed", self.adv[0], offset, || first_val).map(ACell)?;
            let next_cell = region.assign_advice(|| "next value mod m", self.adv[1], offset, || rem_val).map(ACell)?;
            let quot_cell = region.assign_advice(|| "quotient", self.adv[1], offset, || quot_val).map(ACell)?;

            Ok((first_cell, next_cell, quot_cell))
        })
    }

    fn assign_next_modulo (
        &self,
        mut layouter: impl Layouter<F>,
        prev: &ACell<F>,
    ) -> Result<ACell<F>, Error> {
        let offset = 0;

    }
    
    fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        cell: &ACell<F>,
        row: usize,
    ) -> Result<(), Error> {

    }

}



fn main() {
    println!("Hello, world!");
}
