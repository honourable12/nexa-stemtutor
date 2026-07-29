import { writable, derived } from 'svelte/store';

export const currentModule = writable('Mechanics'); // Mechanics, Optics, Thermodynamics, Electromagnetism, Chemistry
export const currentLanguage = writable('EN'); // EN, FR, SW

export const labState = writable({
  experiment: 'simple_pendulum', // simple_pendulum, solenoid, ideal_gas, optics_lens, titration
  length: 1.00, // meters
  angle: 30, // degrees
  damping: 'None',
  gravity: 9.81, // m/s^2
  mass: 1.00, // kg
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
  opticsLensType: 'Convex', // Convex, Concave

  // Chemistry Titration parameters (kept for legacy support if needed)
  titrationAddedVolume: 0.0, // mL
  titrationTitrantConc: 0.10, // M
  titrationAnalyteVolume: 25.0, // mL
  titrationAnalyteConc: 0.10, // M
  titrationIndicator: 'Phenolphthalein', // Phenolphthalein, Methyl Orange, Bromothymol Blue
  titrationTitrant: 'NaOH',
  titrationAnalyte: 'HCl'
});

export const titrationState = writable({
  titrant: 'NaOH',
  analyte: 'HCl',
  addedVolume: 0.0, // mL
  indicator: 'Phenolphthalein',
  titrantConc: 0.10, // M
  analyteVolume: 25.0, // mL
  analyteConc: 0.10 // M
});

export const currentPH = derived(titrationState, ($state) => {
  const vAcid = $state.analyteVolume;
  const cAcid = $state.analyteConc;
  const cBase = $state.titrantConc;
  const vAdded = $state.addedVolume;
  
  const molesAcid = cAcid * (vAcid / 1000);
  const molesBase = cBase * (vAdded / 1000);
  const totalVolL = (vAcid + vAdded) / 1000;
  
  if (molesBase < molesAcid) {
    // Before equivalence point (excess acid)
    const excessAcidMoles = molesAcid - molesBase;
    const hConc = excessAcidMoles / totalVolL;
    const ph = -Math.log10(Math.max(hConc, 1e-14));
    return parseFloat(ph.toFixed(2));
  } else if (Math.abs(molesAcid - molesBase) < 1e-9) {
    // Equivalence point
    return 7.00;
  } else {
    // After equivalence point (excess base)
    const excessBaseMoles = molesBase - molesAcid;
    const ohConc = excessBaseMoles / totalVolL;
    const pOH = -Math.log10(Math.max(ohConc, 1e-14));
    const ph = 14.0 - pOH;
    return parseFloat(ph.toFixed(2));
  }
});

export const chatMessages = writable([
  {
    sender: 'nexa',
    time: '18:42 AM',
    text: 'The period $T$ of a simple pendulum is given by $$T = 2\\pi \\sqrt{\\frac{L}{g}}$$. Notice how the mass doesn\'t affect the period of an ideal pendulum! However, when damping is enabled, a larger mass has more momentum, making it decay slower.'
  },
  {
    sender: 'nexa',
    time: '18:43 AM',
    text: 'The formula we are applying in the simulation is:\n\n$$T = 2\\pi \\sqrt{\\frac{L}{g}}$$\n\n*Note: Use the mass slider to see how it alters the inertia and the rate of energy dissipation when air friction or viscous fluid is active!*'
  }
]);

export const isModuleModalOpen = writable(false);
export const isHardwareModalOpen = writable(false);