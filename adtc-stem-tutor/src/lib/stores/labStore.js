import { writable } from 'svelte/store';

export const currentModule = writable('Mechanics'); // Mechanics, Optics, Thermodynamics, Electromagnetism
export const currentLanguage = writable('EN'); // EN, FR, SW

export const labState = writable({
  experiment: 'simple_pendulum', // simple_pendulum, solenoid, ideal_gas, optics_lens
  length: 1.00, // meters
  angle: 30, // degrees
  damping: 'None',
  gravity: 9.81, // m/s^2
  period: 2.01, // seconds
  
  // Solenoid Electromagnetism parameters
  solenoidTurns: 20,
  solenoidCurrent: 2.0, // Amperes
  solenoidPermeability: 500, // relative permeability mu_r

  // Thermodynamics Ideal Gas parameters
  gasTemperature: 300, // Kelvin
  gasVolume: 1.00, // relative volume factor
  gasParticles: 60,

  // Optics Thin Lens parameters
  opticsObjectDistance: 45, // cm
  opticsFocalLength: 20, // cm
  opticsObjectHeight: 15, // cm
  opticsLensType: 'Convex' // Convex, Concave
});

export const chatMessages = writable([
  {
    sender: 'nexa',
    time: '18:42 AM',
    text: 'The period $T$ of a simple pendulum is given by $$T = 2\\pi \\sqrt{\\frac{L}{g}}$$. Notice how the mass doesn\'t affect the period!'
  },
  {
    sender: 'nexa',
    time: '18:43 AM',
    text: 'The formula we are applying in the simulation is:\n\n$$T = 2\\pi \\sqrt{\\frac{L}{g}}$$\n\n*Note: Since we are using an ideal pivot, friction effects are currently set to None.*'
  }
]);

export const isModuleModalOpen = writable(false);
export const isHardwareModalOpen = writable(false);