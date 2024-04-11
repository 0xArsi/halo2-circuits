#![allow(warnings, unused)]
use halo2_proofs::{
    arithmetic::FieldExt, circuit::{AssignedCell, Layouter, Value}, dev::metadata::Constraint, plonk::{Advice, Assigned, Column, ConstraintSystem, Constraints, Error, Expression, Selector, TableColumn}, poly::Rotation
};

use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct RangeConstrained<F: FieldExt>(AssignedCell<Assigned<F>, F>);

#[derive(Clone, Debug)]
pub struct RangeTableConfig<F: FieldExt, const RANGE: usize>{
    pub value: TableColumn,
    pub _marker: PhantomData<F>,
}

impl<F: FieldExt, const RANGE: usize> RangeTableConfig<F, RANGE>{
    pub fn configure(cs: &mut ConstraintSystem<F>) -> Self {
        let values = cs.lookup_table_column();

        Self{
            value: values,
            _marker: PhantomData::<F>
        }
    }

    pub fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        
        layouter.assign_table(||"assign table", |mut table| {
            for i in (0..RANGE) {
                table.assign_cell(||"assign lookup table value", self.value, i, || Value::known(F::from(i as u64)))?;
            }
            Ok(())
        })
    }
}
#[derive(Clone, Debug)]
pub struct RangeCheckLookupConfig<F: FieldExt, const RANGE: usize>{
    pub values: Column<Advice>,
    pub q_enable: Selector,
    pub table: RangeTableConfig<F, RANGE>,
}

impl<F: FieldExt, const RANGE: usize> RangeCheckLookupConfig<F, RANGE> {
    pub fn configure(cs: &mut ConstraintSystem<F>, values: Column<Advice>) -> Self{
        let q_enable = cs.complex_selector();
        let table = RangeTableConfig::configure(cs);
        cs.lookup(|cs| {
            let q_lookup = cs.query_selector(q_enable);
            let v = cs.query_advice(values, Rotation::cur());
            vec![(q_lookup * v, table.value)]
        });

        Self {
            values: values,
            q_enable: q_enable,
            table: table,
        }
    }

    pub fn assign_lookup(&self, mut layouter: impl Layouter<F>, val: Value<Assigned<F>>) -> Result<RangeConstrained<F>, Error>{
        let offset = 0;
        layouter.assign_region(|| "assign value", |mut region| {
            self.q_enable.enable(&mut region, offset)?;
            region.assign_advice(||"advice", self.values, offset, ||val)
                  .map(RangeConstrained)
        })
    }
}

#[cfg(test)]
mod tests{
    use halo2_proofs::{
        dev::MockProver,
        pasta::Fp,
        circuit::{Value, SimpleFloorPlanner},
        plonk::{Any, Circuit, Assigned, ConstraintSystem},
    };

    use super::*;

    //do it for multiple values not just one
    #[derive(Default)]
    pub struct RangeCheckLookupCircuit<F: FieldExt, const RANGE: usize> {
        pub lookup_values: Vec<Value<Assigned<F>>>,

    }
    impl<F: FieldExt, const RANGE: usize> Circuit<F> for RangeCheckLookupCircuit<F, RANGE> {
        type Config = RangeCheckLookupConfig<F, RANGE>;
        type FloorPlanner = SimpleFloorPlanner;
        fn without_witnesses(&self) -> Self {
            Self::default()
        }
        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            let value = meta.advice_column();
            RangeCheckLookupConfig::configure(meta, value)
        }

        fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
            config.table.load(&mut layouter)?;

            self.lookup_values.iter().for_each(|v| {
                config.assign_lookup(layouter.namespace(||"layout"), *v).unwrap();
            });
            
            Ok(())
        }
    }

    #[test]
    fn test_complete(){
        let k = 4;
        let lookup_values = vec![Value::known(Fp::from(2 as u64)).into(),Value::known(Fp::from(5 as u64).into())];
        const RANGE: usize = 9;
        let circuit = RangeCheckLookupCircuit::<Fp, RANGE> {
            lookup_values: lookup_values
        };
        let prover =MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }
    
    #[test]
    #[should_panic]
    fn test_sound(){
        let k = 4;
        let lookup_values = vec![Value::known(Fp::from(25 as u64)).into(),Value::known(Fp::from(24 as u64).into())];
        const RANGE: usize = 9;
        let circuit = RangeCheckLookupCircuit::<Fp, RANGE> {
            lookup_values: lookup_values
        };
        let prover =MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }
}