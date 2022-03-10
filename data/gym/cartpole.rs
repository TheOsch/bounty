#![allow(unknown_lints, uncommon_codepoints, mixed_script_confusables)]

use fomat_macros::pintln;
use gstuff::re::Re;
use gstuff::{round_to, slurp};
use serde_json as json;
use tch::Tensor;
use tch::{nn, nn::Module, nn::OptimizerConfig, Device, Reduction};

/// Example of a specialized network, reused in a generic network.
#[derive(Debug)]
struct Act2Vel {bias: Tensor, i2o: Tensor}
impl Act2Vel {
  fn new (vs: &nn::Path) -> Act2Vel {
    let bound = 1.0 / (2 as f64) .sqrt();
    Act2Vel {
      bias: vs.var ("bias", &[1], nn::Init::Uniform {lo: -bound, up: bound}),
      i2o: vs.var ("i2o", &[1, 2], nn::Init::KaimingUniform)}}}
impl Module for Act2Vel {
  fn forward (&self, xs: &Tensor) -> Tensor {
    // use just the “previous velocity” and the “action” columns for the “action → velocity” inference
    let xs = xs.index (&[None, Some (&Tensor::of_slice (&[1i64, 4]))]);
    xs.matmul (&self.i2o.tr()) + &self.bias}}

#[derive(Debug)]
struct Net {a2v: Act2Vel, bs: Tensor, i2h: Tensor, h2o: Tensor}
impl Net {
  fn new (vs: &nn::Path) -> Net {
    const HIDDEN: i64 = 5;
    let bound = 1.0 / (5 as f64) .sqrt();
    Net {
      a2v: Act2Vel::new (&(vs / "a2v")),
      bs: vs.var ("bias", &[HIDDEN], nn::Init::Uniform {lo: -bound, up: bound}),
      i2h: vs.var ("i2h", &[HIDDEN, 6], nn::Init::KaimingUniform),
      h2o: vs.var ("h2o", &[4, HIDDEN], nn::Init::KaimingUniform)}}}
impl Module for Net {
  fn forward (&self, xs: &Tensor) -> Tensor {
    let velocity = self.a2v.forward (xs);

    // add separately predicted velocity into inputs
    // cf. https://pytorch.org/docs/stable/generated/torch.cat.html
    let xs = Tensor::cat (&[xs, &velocity], 1);

    let xs = xs.matmul (&self.i2h.tr()) + &self.bs;
    // Learns better without activation.
    //let xs = xs.relu();
    xs.matmul (&self.h2o.tr())}}

fn mainʹ() -> Re<()> {
  let sessions: Vec<(Vec<u8>, Vec<(f32, f32, f32, f32)>)> = json::from_slice (&slurp (&"cartpole.json"))?;
  let mut inputsᵃ = Vec::<f32>::new();
  let mut outputsᵃ = Vec::<f32>::new();
  for (actions, observations) in &sessions {
    for ix in 1 .. actions.len() {
      let obs = &observations[ix-1];
      inputsᵃ.push (obs.0);  // Cart Position
      inputsᵃ.push (obs.1);  // Cart Velocity
      inputsᵃ.push (obs.2);  // Pole Angle
      inputsᵃ.push (obs.3);  // Pole Angular Velocity
      inputsᵃ.push (actions[ix] as f32);
      outputsᵃ.push (observations[ix].0);  // Cart Position
      outputsᵃ.push (observations[ix].1);  // Cart Velocity
      outputsᵃ.push (observations[ix].2);  // Pole Angle
      outputsᵃ.push (observations[ix].3)}}  // Pole Angular Velocity
  let inputs = Tensor::of_slice (&inputsᵃ) .view((inputsᵃ.len() as i64 / 5, 5));
  let outputs = Tensor::of_slice (&outputsᵃ) .view((outputsᵃ.len() as i64 / 4, 4));

  let velocity_outputs = outputs.index (&[None, Some (&Tensor::of_slice (&[1i64]))]);

  let vs = nn::VarStore::new (Device::Cpu);
  let net = Net::new (&vs.root());
  let mut opt = nn::Adam::default().build (&vs, 0.1)?;
  opt.set_weight_decay (0.01);
  for epoch in 1 ..= 2022 {
    let a2v_loss = net.a2v.forward (&inputs) .mse_loss (&velocity_outputs, Reduction::Sum);
    opt.backward_step (&a2v_loss);
    let a2v_lossᶠ = f32::from (&a2v_loss);

    let loss = net.forward (&inputs) .mse_loss (&outputs, Reduction::Sum);
    opt.backward_step (&loss);
    let lossᶠ = f32::from (&loss);
    pintln! ("epoch " {"{:>4}", epoch} " "
      " a2v_loss " {"{:<7}", round_to (3, a2v_lossᶠ)}
      " loss " {"{:<7}", round_to (3, lossᶠ)});
    if lossᶠ < 0.1 {break}}

  for ix in 0..13 {
    let input = Tensor::of_slice (&inputsᵃ[ix * 5 .. (ix + 1) * 5]) .view((1, 5));
    let velocity = net.a2v.forward (&input);
    let prediction = net.forward (&input);
    pintln! ("velocity expected " {"{:>7.4}", outputsᵃ[ix * 4 + 1]}
      " a2v " {"{:>7.4}", f32::from (velocity)}
      " net " {"{:>7.4}", f32::from (prediction.get (0) .get (1))})}

  pintln! ("--- a2v i2o ---");
  net.a2v.i2o.print();
  pintln! ("--- a2v bias ---");
  net.a2v.bias.print();

  // ⌥ Train the velocity formula directly with Adam,
  // velocity = (previous_velocity * 1.0010 + action * 0.3900) - 0.1950
  // velocity = previous_velocity + action ? 0.2 : -0.2

  Re::Ok(())}

fn adam2plus2() -> Re<()> {
  // cf. https://arxiv.org/pdf/1412.6980.pdf Adam: a method for stochastic optimization

  // Adam is a “mathematical optimization” with the goal of minimizing a function.
  // Most of the time the function we want to minimize is the loss function:
  // the smaller the loss, the better the fitness of the parameters picked (aka model).
  // For “2 + x = 4” the loss function would be “(2 - x) ^ 2”.
  fn loss (x: f32) -> f32 {(2. - x) .powf (2.)}

  // “The gradient always points in the direction of steepest increase in the loss function.”
  // In Autograd the gradient is calculated automatically together with the loss
  // and is consequently reused by the optimization algorithm.
  // Another popular option seems to be in implementing the gradient as derivative of the loss.
  // “The derivative of a function y = f(x) of a variable x is a measure of the rate
  // at which the value y of the function changes with respect to the change of the variable x.”
  // For `a^2` [the known derivative is `2a`](https://en.wikipedia.org/wiki/Derivative#Example).
  fn dloss (x: f32) -> f32 {2. * (2. - x)}
  // We might be able to calculate it as the difference between the current and the previous loss.
  // cf. https://en.wikipedia.org/wiki/Numerical_differentiation

  let α = 0.001;
  let β1 = 0.9;
  let β2 = 0.999;
  let ε = 0.1;
  let mut m = 0.0;
  let mut v = 0.0;
  let mut t = 0;
  let mut θ = 0f32;

  loop {
    t += 1;

    let g = -dloss (θ);
    m = β1 * m + (1. - β1) * g;
    v = β2 * v + (1. - β2) * g .powi (2);
    let mˆ = m / (1. - β1 .powi (t));
    let vˆ = v / (1. - β2 .powi (t));
    θ = θ - α * mˆ / (vˆ.sqrt() + ε);
    let loss = loss (θ);
    if loss < 0.01 {break}  // stop if converged
    if t % 1000 == 0 {pintln! ([=t] ' ' [=θ] ' ' [=g] ' ' [=loss])}}

  pintln! ("Converged in " (t) " steps; " [=θ]);
  Re::Ok(())}

fn main() {
  adam2plus2().unwrap(); return;
  mainʹ().unwrap()}
