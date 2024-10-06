//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --release -- --execute
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run --release -- --prove
//! ```

use rand::{rngs::StdRng, Rng, SeedableRng};

use clap::Parser;
use prism_common::test_utils::{create_random_insert, create_random_update, TestTreeState};
use prism_common::tree::{Batch, Digest, Proof};
use sp1_sdk::{ProverClient, SP1Stdin};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const FIBONACCI_ELF: &[u8] = include_bytes!("../../../elf/riscv32im-succinct-zkvm-elf");

/// The arguments for the command.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    execute: bool,

    #[clap(long)]
    prove: bool,
}

fn create_automated_batch(initial_leaf_count: usize, num_operations: usize) -> Batch {
    let mut state = TestTreeState::new();
    let mut rng = StdRng::from_entropy();
    let mut proofs = Vec::with_capacity(num_operations);

    println!("Prefilling tree...");
    for _ in 0..initial_leaf_count {
        let _ = create_random_insert(&mut state, &mut rng);
        // proofs.push(Proof::Insert(proof));
    }
    println!("Prefilled tree");

    let prev_root = state.tree.get_current_root().unwrap();

    for _ in 0..num_operations {
        if rng.gen_bool(0.7) {
            let proof = create_random_update(&mut state, &mut rng);
            proofs.push(Proof::Update(proof));
        } else {
            let proof = create_random_insert(&mut state, &mut rng);
            proofs.push(Proof::Insert(proof));
        }
    }

    let new_root = state.tree.get_current_root().unwrap();

    println!("done");
    Batch {
        prev_root: Digest::new(prev_root.into()),
        new_root: Digest::new(new_root.into()),
        proofs,
    }
}

fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    // Parse the command line arguments.
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    // Setup the prover client.
    let client = ProverClient::new();

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    let batch = create_automated_batch(1, 100);
    stdin.write(&batch);

    if args.execute {
        // Execute the program
        let (output, report) = client.execute(FIBONACCI_ELF, stdin).run().unwrap();
        println!("Program executed successfully.");

        // Read the output.
        let decoded = output.as_slice();
        let final_commitment = hex::encode(decoded);
        println!("final_commitment: {}", final_commitment);

        assert_eq!(final_commitment, batch.new_root.to_hex());
        println!("Values are correct!");

        // Record the number of cycles executed.
        println!("Number of cycles: {}", report.total_instruction_count());
    } else {
        // Setup the program for proving.
        let (pk, vk) = client.setup(FIBONACCI_ELF);

        // Generate the proof
        let proof = client
            .prove(&pk, stdin)
            .groth16()
            .run()
            .expect("failed to generate proof");

        println!("Successfully generated proof!");

        // Verify the proof.
        client.verify(&proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");
    }
}
