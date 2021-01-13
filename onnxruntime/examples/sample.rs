#![forbid(unsafe_code)]

use onnxruntime::{
    environment::Environment,
    ndarray::{Array, Ix4, IxDyn},
    runner::{Outputs, Runner},
    tensor::OrtOwnedTensor,
    GraphOptimizationLevel, LoggingLevel,
};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

type Error = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    // Setup the example's log level.
    // NOTE: ONNX Runtime's log level is controlled separately when building the environment.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let environment = Environment::builder()
        .with_name("test")
        // The ONNX Runtime's log level can be different than the one of the wrapper crate or the application.
        .with_log_level(LoggingLevel::Info)
        .build()?;

    let mut session = environment
        .new_session_builder()?
        .with_optimization_level(GraphOptimizationLevel::Basic)?
        .with_number_threads(1)?
        // NOTE: The example uses SqueezeNet 1.0 (ONNX version: 1.3, Opset version: 8),
        //       _not_ SqueezeNet 1.1 as downloaded by '.with_model_downloaded(ImageClassification::SqueezeNet)'
        //       Obtain it with:
        //          curl -LO "https://github.com/onnx/models/raw/master/vision/classification/squeezenet/model/squeezenet1.0-8.onnx"
        .with_model_from_file("squeezenet1.0-8.onnx")?;

    let input0_shape: Vec<usize> = session.inputs[0].dimensions().map(|d| d.unwrap()).collect();
    let output0_shape: Vec<usize> = session.outputs[0]
        .dimensions()
        .map(|d| d.unwrap())
        .collect();

    assert_eq!(input0_shape, [1, 3, 224, 224]);
    assert_eq!(output0_shape, [1, 1000, 1, 1]);

    // initialize input data with values in [0.0, 1.0]
    let n: u32 = session.inputs[0]
        .dimensions
        .iter()
        .map(|d| d.unwrap())
        .product();
    let array = Array::linspace(0.0_f32, 1.0, n as usize)
        .into_shape(input0_shape)
        .unwrap();
    let input_tensor_values = vec![array];

    // You can simply run the session with the input to get the output...
    // let outputs: Vec<OrtOwnedTensor<f32, _>> = session.run(input_tensor_values)?;

    // Or, you can build a runner to pre-allocate the output
    let mut runner = session
        .make_runner(input_tensor_values)
        .with_output::<f32, Ix4>()?;
    runner.execute()?;

    print_runner_outputs(&runner);

    // Since the runner now owns the input and keep it alive, we can access it
    // and modify it without reallocation.
    *(&mut runner.inputs()[0]) *= 2.0f32;
    runner.execute()?;

    print_runner_outputs(&runner);

    Ok(())
}

fn print_runner_outputs(runner: &Runner<f32, IxDyn, f32, Ix4>) {
    let outputs: Outputs<f32, Ix4> = runner.outputs();
    let output = &outputs[0];
    for i in 0..5 {
        println!("Score for class [{}] =  {}", i, output[[0, i, 0, 0]]);
    }
}
