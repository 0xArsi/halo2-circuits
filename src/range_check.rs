#![allow(warnings, unused)]
use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Value, AssignedCell, Layouter},
    plonk::{Advice, Assigned, Column, ConstraintSystem, Constraints, Expression, Selector, Error}, poly::Rotation,
};



#[derive(Debug, Clone)]
struct RangeConstrained<F: FieldExt>(AssignedCell<Assigned<F>, F>);

#[derive(Debug, Clone)]
pub struct RangeCheckCircuitConfig<F: FieldExt, const RANGE_SIZE: usize>{
    //what values we want to range check
    pub value: Column<Advice>,
    //selector to enable/disable some values from being checked
    pub q_enable: Selector,
    //number of elements in range

    _marker: PhantomData<F>,
}


impl <F: FieldExt, const RANGE_SIZE: usize> RangeCheckCircuitConfig<F, RANGE_SIZE>{
    pub fn configure(cs: &mut ConstraintSystem<F>, value: Column<Advice>) -> Self{
        //make selector columns
        let q_select = cs.selector();
        //make advice column to put the value(s) in
        cs.create_gate(
            "range",
            |cs| {
                //query the value of the selector
                let q_select = cs.query_selector(q_select);

                //query the value at the current position
                let value = cs.query_advice(value, Rotation::cur());

                //check that value is in range by multiplying its differences with every value
                //one of them has to be zero if it is in the range

                let range_check = |range: usize, value: Expression<F>| {
                    assert!(range > 0);
                    (1..range).fold(
                        value.clone(),
                        |expr, i|{
                        expr * (Expression::Constant(F::from(i as u64)) - value.clone())
                    })
                };
                Constraints::with_selector(q_select, [("range check", range_check(RANGE_SIZE, value))])    
            }
        );
        Self { value: value, q_enable: q_select, _marker:PhantomData::<F> }
    }

    pub fn assign(&self, mut layouter: impl Layouter<F>, value: Value<Assigned<F>>) -> Result<RangeConstrained<F>, Error>{
        let offset = 0;
        layouter.assign_region(
            || "assign range val",
            |mut region| {
                self.q_enable.enable(&mut region, offset)?;
                region.assign_advice(||"value", self.value, offset, ||value)
                .map(RangeConstrained)

            }
        )

    } 
}

#[cfg(test)]
mod tests{
    use halo2_proofs::{
        circuit::SimpleFloorPlanner,
        dev::{FailureLocation, MockProver, VerifyFailure},
        pasta::Fp,
        plonk::{Any, Circuit},
    }; 
    use super::*;

    #[derive(Default)]
    struct RangeCheckCircuit<F: FieldExt, const RANGE_SIZE: usize>{
        pub value: Value<Assigned<F>>,
    }

    impl<F: FieldExt, const RANGE_SIZE: usize> Circuit<F> for RangeCheckCircuit<F, RANGE_SIZE> {
        type Config = RangeCheckCircuitConfig<F, RANGE_SIZE>;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self{
            Self::default()
        }

        fn configure(cs: &mut ConstraintSystem<F>) -> Self::Config{
            //create advice column to give to configuration
            let value = cs.advice_column();
            Self::Config::configure(cs, value)
        }

        fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
            config.assign(layouter.namespace(
                ||"Assign value to test circ"), 
                self.value
            )?;
            Ok(())
        }
    }
    #[test]
    fn test_range_check_complete(){
        let k = 4;
        const range_size: usize = 8;


        //check that prover produces circuit that gets acccepted when the value is in range\
        for i in (0..range_size){
            let circuit = RangeCheckCircuit::<Fp, range_size>{
                value: Value::known(Fp::from(i as u64).into())
            };
            
            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            prover.assert_satisfied();
        }

    
    }
    #[test]
    #[should_panic]
    fn test_range_check_sound(){
        let k = 4;
        const range_size: usize = 8;
        let circuit = RangeCheckCircuit::<Fp, range_size>{
            value: Value::known(Fp::from(range_size as u64).into())
        };
        
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.verify().unwrap();
    }
}