// use clap::Parser;
// use deno_core::Extension;
// use deno_core::JsRuntime;
// use deno_core::RuntimeOptions;
// use deno_core::{op, v8};
// use std::env;
// use std::fs;
// use tokio::runtime::Builder;

// #[op]
// fn op_sum(nums: Vec<f64>) -> Result<f64, deno_core::error::AnyError> {
// 	let sums = nums.iter().fold(0.0, |acc, v| acc + v);
// 	Ok(sums)
// }

// #[derive(Parser, Debug)]
// #[clap(author, version, about, long_about = None)]
// struct Args {
// 	#[clap(short, long)]
// 	filename: String,
// 	// todo allow args to be passed to the script
// }

fn main() {
	// let args = Args::parse();

	opal::main();

	// let args: Vec<String> = std::env::args().collect();
	// if args.len() < 2 {
	// 	println!("Usage: rustjs <script>");
	// 	std::process::exit(1);
	// }

	// // build extension with custom ops
	// let ext = Extension::builder().ops(vec![op_sum::decl()]).build();

	// // create runtime with custom extension
	// let mut js_runtime = JsRuntime::new(RuntimeOptions {
	// 	extensions: vec![ext],
	// 	..Default::default()
	// });

	// let runtime = tokio::runtime::Builder::new_current_thread()
	// 	.enable_all()
	// 	.build()
	// 	.unwrap();

	// let source_code =
	// 	fs::read_to_string(args.filename).expect("Something went wrong reading the file");

	// let future = async move {
	// 	// load js script
	// 	js_runtime.execute_script("script", &*source_code).unwrap();
	// 	js_runtime.run_event_loop(false).await
	// };
	// runtime.block_on(future).unwrap();
}
