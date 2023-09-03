use halo2_proofs::{
    plonk::{ConstraintSystem, Error, Column, Advice, Selector, Instance, Expression, Circuit},
    circuit::{Layouter, Value, SimpleFloorPlanner, AssignedCell},
    poly::Rotation, pasta::{Fp, group::ff::PrimeField}, dev,
};

struct ACell (AssignedCell<Fp,Fp>);

// MODULUS_EXPONENT should be multiple of 8
// MODULUS^2 should be less than the order of prime field
// below are constants of Linear Congruence generator in C++
pub const MODULUS_EXPONENT: u64 = 32;
pub const MODULUS: u64 = 1 << MODULUS_EXPONENT;
pub const MULTIPLIER: u64 = 214013;
pub const INCREMENT: u64 = 2531011;

#[derive(Clone, Debug)]
struct GachaConfig<const NUMBER_OF_ITER: u64> {
    adv: [Column<Advice>; 2],
    divisor: Column<Advice>,
    inst: Column<Instance>,
    selector: Selector,
}

impl<const NUMBER_OF_ITER: u64> GachaConfig<NUMBER_OF_ITER> {
    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self {
        let adv_0 = meta.advice_column();
        let adv_1 = meta. advice_column();
        let divisor = meta.advice_column();
        let selector = meta.selector();
        let inst = meta.instance_column();

        meta.enable_equality(adv_0);
        meta.enable_equality(adv_1);
        meta.enable_equality(inst);

        meta.create_gate("linear congruence", |meta| {
            let x = meta.query_advice(adv_0, Rotation::cur());
            let b = meta.query_advice(adv_1, Rotation::cur());
            let d = meta.query_advice(divisor, Rotation::cur());

            let a = Expression::Constant(Fp::from(MULTIPLIER));
            let m = Expression::Constant(Fp::from(MODULUS));
            let c = Expression::Constant(Fp::from(INCREMENT));

            let s = meta.query_selector(selector);

            vec![s * (a * x + c - b - m * d)]
        });

        GachaConfig {
            adv: [adv_0, adv_1],
            divisor: divisor, 
            inst: inst,
            selector: selector,
        }
    }

    fn next_value(prev_val: Value<Fp>) -> Value<Fp> {
        prev_val.map(|a| {
            a * Fp::from(MULTIPLIER) + Fp::from(INCREMENT)
        })
    }

    fn assign_first_row(
        &self,
        mut layouter: impl Layouter<Fp>,
        seed: Value<Fp>,
    ) -> Result<ACell, Error> {
        layouter.assign_region(|| "linear operation", |mut region| {
            let offset = 0;

            self.selector.enable(&mut region, offset)?;

            let seed_val = seed.map(rem);
            let next_val = Self::next_value(seed_val);
            let rem_val = next_val.map(rem);
            let quot_val = next_val.map(quot);      

            region.assign_advice(|| "seed", self.adv[0], offset, || seed_val).map(ACell)?;
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
            let next_val = Self::next_value(prev_val);
            let rem_val = next_val.map(rem);
            let quot_val = next_val.map(quot);

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
struct GachaCircuit<const NUMBER_OF_ITER: u64> {
    seed: Value<Fp>,
}


impl<const NUMBER_OF_ITER: u64> Circuit<Fp> for GachaCircuit<NUMBER_OF_ITER> {
    type Config = GachaConfig<NUMBER_OF_ITER>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        GachaConfig::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<Fp>) -> Result<(), Error> {
        let mut prev = config.assign_first_row(layouter.namespace(|| "first row"), self.seed)?;

        for _i in 1..NUMBER_OF_ITER {
            prev = config.assign_next_row(layouter.namespace(|| "next row"), &prev)?;
        }

        config.expose_public(layouter.namespace(|| "out"), &prev, 0)?;
        
        Ok(())
    }
}

// divide field element (as integer) with MODULUS
fn rem(input: Fp) -> Fp {
    let divisor: usize = (MODULUS_EXPONENT / 8).try_into().unwrap();
    let repr = input.to_repr();
    let mut rem_repr: [u8; 32] = [0; 32];
    for i in 0..divisor {
        rem_repr[i] = repr[i];
    }
    Fp::from_repr(rem_repr).unwrap()
}

fn quot(input: Fp) -> Fp {
    let divisor: usize = (MODULUS_EXPONENT / 8).try_into().unwrap();
    let repr = input.to_repr();
    let mut ret_repr: [u8; 32] = [0; 32];
    for i in 0..(32-divisor) {
        ret_repr[i] = repr[i+divisor];
    }
    Fp::from_repr(ret_repr).unwrap()
}

fn get_random(
    seed: u64,
    number_of_iter: u64,
) -> u64 {
    let mut ret = seed % MODULUS;
    for _i in 0..number_of_iter {
        ret = (ret * MULTIPLIER + INCREMENT) % MODULUS;
    }
    ret
}


#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{
        dev::MockProver,
        pasta::Fp,
    };

    #[test]
    fn test_rand() {
        let seed: u64 = 54352;
        const NUMBER_OF_ITER: u64 = 100;
        
        let circuit = GachaCircuit::<NUMBER_OF_ITER> {
            seed: Value::known(Fp::from(seed)),
        };
        
        let rand = get_random(seed, NUMBER_OF_ITER);
        println!("{}", rand);
        let public_input = vec![Fp::from(rand)];
        let prover = MockProver::run(10, &circuit, vec![public_input]).unwrap();
        prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn print_test_rand() {
        use plotters::prelude::*;

        let seed: u64 = 54352;
        const NUMBER_OF_ITER: u64 = 100;
        
        let circuit = GachaCircuit::<NUMBER_OF_ITER> {
            seed: Value::known(Fp::from(seed)),
        };
    
        let root = BitMapBackend::new("rand.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Rand Layout", ("sans-serif", 60)).unwrap();

        halo2_proofs::dev::CircuitLayout::default()
            .render(10, &circuit, &root)
            .unwrap();
    }
}



fn main() {
    use dev::MockProver;
    let seed: u64 = 543657876595952;
    
    println!("random number check:");
    for i in 1..30 {
        print!("{} ", get_random(seed, i));
    }

    const NUMBER_OF_ITER: u64 = 100;
    
    println!("\ncircuit check:");
    let rand = get_random(seed, 100);
    println!("{}", rand);

    let circuit = GachaCircuit::<NUMBER_OF_ITER> {
        seed: Value::known(Fp::from(seed)),
    };
    let public_input = vec![Fp::from(rand)];
    let prover = MockProver::run(10, &circuit, vec![public_input]).unwrap();
    prover.assert_satisfied();
    println!("sucsess");
}
