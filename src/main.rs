use halo2_proofs::{
    plonk::{ConstraintSystem, Error, Column, Advice, Selector, Instance, Expression, Circuit},
    circuit::{Layouter, Value, SimpleFloorPlanner, AssignedCell},
    poly::Rotation, pasta::{Fp, group::ff::PrimeField},
};

struct ACell (AssignedCell<Fp,Fp>);

#[derive(Clone, Debug)]
struct GachaConfig<const MULTIPLIER: u64, const ADDER: u64> {
    adv: [Column<Advice>; 2],
    divisor: Column<Advice>,
    inst: Column<Instance>,
    selector: Selector,
}

fn rem(input: Fp) -> Fp {
    let repr = input.to_repr();
    let mut rem_repr: [u8; 32] = [0; 32];
    rem_repr[0] = repr[0];
    rem_repr[1] = repr[1];
    Fp::from_repr(rem_repr).unwrap()
}

fn quot(input: Fp) -> Fp {
    let mut repr = input.to_repr();
    repr[0] = 0;
    repr[1] = 0;
    for i in 0..30 {
        repr[i] = repr[i+2];
    }
    Fp::from_repr(repr).unwrap()
}

impl<const MULTIPLIER: u64, const ADDER: u64> GachaConfig<MULTIPLIER, ADDER> {
    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self {
        let adv_0 = meta.advice_column();
        let adv_1 = meta. advice_column();
        let divisor = meta.advice_column();
        let selector = meta.selector();
        let inst = meta.instance_column();

        meta.enable_equality(adv_0);
        meta.enable_equality(adv_1);

        meta.create_gate("linear congruence", |meta| {
            let x = meta.query_advice(adv_0, Rotation::cur());
            let b = meta.query_advice(adv_1, Rotation::cur());
            let d = meta.query_advice(divisor, Rotation::cur());

            let a = Expression::Constant(Fp::from(MULTIPLIER));
            let m = Expression::Constant(Fp::from(65536));
            let c = Expression::Constant(Fp::from(ADDER));

            let s = meta.query_selector(selector);

            vec![ s * (a * x - b - m * d) ]
        });

        GachaConfig {
            adv: [adv_0, adv_1],
            divisor: divisor, 
            inst: inst,
            selector: selector,
        }
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

    

    fn assign_first_row(
        &self,
        mut layouter: impl Layouter<Fp>,
        prev: Value<Fp>,
    ) -> Result<ACell, Error> {
        layouter.assign_region(|| "linear operation", |mut region| {
            let offset = 0;

            self.selector.enable(&mut region, offset)?;

            let rem_val = prev.map(rem);
            let quot_val = prev.map(quot);      

            region.assign_advice(|| "seed", self.adv[0], offset, || prev).map(ACell)?;
            let next_cell = region.assign_advice(|| "next value mod m", self.adv[1], offset, || rem_val).map(ACell)?;
            region.assign_advice(|| "quotient", self.divisor, offset, || quot_val).map(ACell)?;

            Ok(next_cell)
        })
    }

    fn assign_next_row (
        &self,
        mut layouter: impl Layouter<Fp>,
        prev: &ACell,
    ) -> Result<ACell, Error> {
        layouter.assign_region(|| "linear operation", |mut region| {
            let offset = 0;

            self.selector.enable(&mut region, offset)?;

            let prev_val = prev.0.value().copied();
            let rem_val = prev_val.map(rem);
            let quot_val = prev_val.map(quot);      

            prev.0.copy_advice(|| "prev", &mut region, self.adv[0], offset)?;
            let next_cell = region.assign_advice(|| "next value mod m", self.adv[1], offset, || rem_val).map(ACell)?;
            region.assign_advice(|| "quotient", self.divisor, offset, || quot_val).map(ACell)?;

            Ok(next_cell)
        })
    }
    
    fn expose_public(
        &self,
        mut layouter: impl Layouter<Fp>,
        cell: &ACell,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell.0.cell(), self.inst, row)
    }

}

#[derive(Debug, Default)]
struct GachaCircuit<const MULTIPLIER: u64, const ADDER: u64> {
    seed: Value<Fp>,
    n: u64,
}

impl<const MULTIPLIER: u64, const ADDER: u64> Circuit<Fp> for GachaCircuit<MULTIPLIER, ADDER> {
    type Config = GachaConfig<MULTIPLIER, ADDER>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        GachaConfig::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<Fp>) -> Result<(), Error> {
        let mut prev = config.assign_first_row(layouter.namespace(|| "first row"), self.seed)?;

        for _i in 0..self.n {
            let prev = config.assign_next_row(layouter.namespace(|| "next row"), &prev)?;
        }

        config.expose_public(layouter.namespace(|| "out"), &prev, 0)?;
        
        Ok(())
    }




}



fn main() {
    println!("Hello, world!");
    let a = Fp::from(1249842598);
    println!("quot: {:?}", quot(a));
    println!("rem: {:?}", rem(a));

}
