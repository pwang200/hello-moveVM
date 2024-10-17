#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]

mod db;

use std::str::FromStr;
use move_vm_runtime::{
    config::VMConfig, module_traversal::*, move_vm::MoveVM, //native_functions::NativeFunction,
};
use move_bytecode_verifier::VerifierConfig;
// use move_vm_test_utils::InMemoryStorage;
use move_stdlib::{
    natives::{all_natives, nursery_natives, GasParameters, NurseryGasParameters},
    // path_in_crate,
};
use move_core_types::{account_address::AccountAddress};//, effects::ChangeSet};
use alloy::hex;
use move_core_types::language_storage::{ModuleId};//, TypeTag};
use move_core_types::identifier::Identifier;
use move_core_types::value::MoveValue;
use move_vm_types::{gas::UnmeteredGasMeter};//, natives::function::NativeResult};
use crate::db::MockStorage;

fn main() {
    let code = "a11ceb0b0600000008010002020204030605050b04070f1a0829200a49050c4e0f0000000108000002000100020c0300094261736963436f696e04436f696e046d696e740576616c7565000000000000000000000000000000000000000000000000000000000000cafe00020103030001040001050e000b0112002d000200";
    let bytecode = hex::decode(code).unwrap();

    let publisher = AccountAddress::from_hex_literal("0xCAFE").unwrap();

    let mut natives = all_natives(
        AccountAddress::from_hex_literal("0x1").unwrap(),
        GasParameters::zeros(), );
    natives.extend(nursery_natives(
        AccountAddress::from_hex_literal("0x1").unwrap(),
        NurseryGasParameters::zeros(), ));

    let vm = MoveVM::new_with_config(natives, VMConfig {
        verifier: VerifierConfig {
            max_loop_depth: Some(2),
            ..Default::default()
        },
        ..Default::default()
    }).unwrap();

    let storage = MockStorage::new();
    let mut sess = vm.new_session(&storage);
    match sess.publish_module(bytecode, publisher, &mut UnmeteredGasMeter) {
        Ok(_) => {
            println!("publish_module done");
        }
        Err(e) => {
            println!("error : {:?}", e);
        }
    }

    let module_id = ModuleId { address: publisher, name: Identifier::from_str("BasicCoin").unwrap() };
    let function_name = Identifier::from_str("mint").unwrap();
    let ty_args = vec![];//TypeTag::Signer, TypeTag::U64];
    let args = vec![MoveValue::Signer(publisher), MoveValue::U64(10)];
    let args: Vec<_> = args
        .into_iter()
        .map(|val| val.simple_serialize().unwrap())
        .collect();
    let traversal_storage = TraversalStorage::new();
    match sess.execute_entry_function(
        &module_id,
        &function_name,
        ty_args,
        args,
        &mut UnmeteredGasMeter,
        &mut TraversalContext::new(&traversal_storage),
    ) {
        Ok(_) => { println!("function call done"); }
        Err(e) => { println!("error : {:?}", e); }
    }
}
