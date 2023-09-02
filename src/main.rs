use halo2_proofs::{
    plonk::{ConstraintSystem, Error, Column, Advice, Fixed, Selector, Instance},
    arithmetic::Field,
    circuit::{Layouter, Value, SimpleFloorPlanner, AssignedCell},
    poly::Rotation,
};

use std::{marker::PhantomData, os::windows::prelude::IntoRawSocket, ops::Mul};

struct ACell<F: Field> (AssignedCell<F,F>);

struct GachaConfig<F: Field> {
    adv: [Column<Advice>; 2],
    modulus: Column<Fixed>,
    multiplier: Column<Fixed>,
    adder: Column<Fixed>,
    divisor: Column<Advice>,
    selector: Selector,
    _marker: PhantomData<F>,
}

impl<F: Field> GachaConfig<F> {
    fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let adv_0 = meta.advice_column();
        let adv_1 = meta. advice_column();
        let modulus = meta.fixed_column();
        let multiplier = meta.fixed_column();
        let adder = meta.fixed_column();
        let divisor = meta.advice_column();
        let selector = meta.selector();

        meta.enable_equality(adv_0);
        meta.enable_equality(adv_1);

        meta.enable_constant(modulus);
        meta.enable_constant(multiplier);
        meta.enable_constant(adder);

        meta.create_gate("linear congruence", |meta| {
            let x = meta.query_advice(adv_0, Rotation::cur());
            let b = meta.query_advice(adv_1, Rotation::cur());

            let a = meta.query_fixed(multiplier);
            let m = meta.query_fixed(modulus);

            let d = meta.query_advice(divisor, Rotation::cur());

            let s = meta.query_selector(selector);

            vec![ s * (a * x - b - m * d) ]
        });

        GachaConfig {
            adv: [adv_0, adv_1],
            modulus: modulus,
            multiplier: multiplier, 
            adder: adder, 
            divisor: divisor, 
            selector: selector, 
            _marker: PhantomData
        }
    }

    fn assign_first_row(
        &self,
        mut layouter: impl Layouter<F>,
        seed: Value<F>,
        multiplier: Value<F>,
        adder: Value<F>,
    ) -> Result<(ACell<F>, ACell<F>), Error> {
        layouter.assign_region(|| "first row with seed", |mut region| {
            let offset = 0;

            self.selector.enable(&mut region, offset)?;

            let first_cell = region.assign_advice(|| "seed", self.adv[0], offset, || seed).map(ACell)?;
            let next_val = first_cell.0.value().copied().mul(multiplier) + adder;
            let next_cell = region.assign_advice(|| "next value", self.adv[1], offset, || next_val).map(ACell)?;

            region.assign_fixed(|| "multiplier", self.multiplier, offset, || multiplier);
            region.assign_fixed(|| "adder", self.adder, offset, || adder);

            region.assign_advice(|| "divisor", self.divisor, offset, || Value::from(Field::ONE));


            Ok((first_cell, next_cell))
        })
    }

    fn assign_next_row(
        &self,
        mut layouter: impl Layouter<F>,
        prev: &ACell<F>,
    ) -> Result<ACell<F>, Error> {
        layouter.assign_region(|| "linear operation", |mut region| {
            let offset = 0;

            prev.0.copy_advice(|| "prev", &mut region, self.adv[0], offset)?;
            let next_val = prev.0.value().copied() * self.multiplier + self.adder;
            let next_cell = region.assign_advice(|| "next value", self.adv[1], offset, || next_val).map(ACell)?;

            Ok(next_cell)
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
