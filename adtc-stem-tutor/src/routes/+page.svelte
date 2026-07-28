<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { 
    currentModule, 
    currentLanguage, 
    labState, 
    chatMessages, 
    isModuleModalOpen, 
    isHardwareModalOpen 
  } from '$lib/stores/labStore';

  // Import components
  import KaTeX from '$lib/components/KaTeX.svelte';
  import ModuleModal from '$lib/components/ModuleModal.svelte';
  import HardwareModal from '$lib/components/HardwareModal.svelte';

  // Import simulations
  import PendulumSim from '$lib/components/simulations/PendulumSim.svelte';
  import SolenoidSim from '$lib/components/simulations/SolenoidSim.svelte';
  import GasSim from '$lib/components/simulations/GasSim.svelte';
  import OpticsSim from '$lib/components/simulations/OpticsSim.svelte';

  let userInput = "";
  let isGenerating = false;

  // Reactively sync currentModule to labState experiment
  $: {
    if ($currentModule === 'Mechanics') {
      $labState.experiment = 'simple_pendulum';
    } else if ($currentModule === 'Optics') {
      $labState.experiment = 'optics_lens';
    } else if ($currentModule === 'Thermodynamics') {
      $labState.experiment = 'ideal_gas';
    } else if ($currentModule === 'Electromagnetism') {
      $labState.experiment = 'solenoid';
    }
  }

  // Reactively calculate period when L or g changes for pendulum
  $: {
    const L = $labState.length;
    const g = $labState.gravity;
    $labState.period = parseFloat((2 * Math.PI * Math.sqrt(L / g)).toFixed(2));
  }

  // Handle stream from Tauri backend
  onMount(() => {
    let unlisten: (() => void) | undefined;
    
    listen<{ token: string }>('adtc-token-stream', (event) => {
      chatMessages.update(msgs => {
        let lastMsg = msgs[msgs.length - 1];
        if (lastMsg && lastMsg.sender === 'nexa' && isGenerating) {
          lastMsg.text += event.payload.token;
          return [...msgs];
        } else {
          isGenerating = false;
          return msgs;
        }
      });
    }).then(fn => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  });

  async function sendPrompt(customText: string | null = null) {
    const textToSend = customText || userInput;
    if (!textToSend.trim()) return;

    const now = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    chatMessages.update(m => [...m, { sender: 'user', time: now, text: textToSend }]);
    
    if (!customText) userInput = "";
    isGenerating = true;

    chatMessages.update(m => [...m, { sender: 'nexa', time: now, text: "" }]);

    // Build context info based on active experiment/parameters
    let labContextInfo = "";
    if ($currentModule === 'Mechanics') {
      labContextInfo = `Current Experiment: Simple Pendulum. Active parameters: length (L) = ${$labState.length.toFixed(2)} meters, initial swing angle (theta) = ${$labState.angle} degrees, damping = ${$labState.damping}, gravity (g) = ${$labState.gravity.toFixed(2)} m/s^2, mass (m) = ${$labState.mass.toFixed(2)} kg, calculated theoretical period (T) = ${$labState.period} seconds.`;
    } else if ($currentModule === 'Optics') {
      const isConcave = $labState.opticsLensType === 'Concave';
      const f = isConcave ? -$labState.opticsFocalLength : $labState.opticsFocalLength;
      const doVal = $labState.opticsObjectDistance;
      const ho = $labState.opticsObjectHeight;
      let diStr = "at infinity";
      let magnification = "infinite";
      let imageType = "none";
      if (isConcave) {
        const diVal = (f * doVal) / (doVal - f);
        diStr = `${diVal.toFixed(1)} cm`;
        magnification = (-diVal / doVal).toFixed(2);
        imageType = "Virtual (Upright, Diminished)";
      } else {
        if (Math.abs(doVal - f) > 0.05) {
          const diVal = (f * doVal) / (doVal - f);
          diStr = `${diVal.toFixed(1)} cm`;
          magnification = (-diVal / doVal).toFixed(2);
          imageType = diVal > 0 ? "Real (Inverted)" : "Virtual (Upright, Magnified)";
        }
      }
      labContextInfo = `Current Experiment: Thin ${$labState.opticsLensType} Lens Optics. Active parameters: object distance (d_o) = ${doVal} cm, focal length (f) = ${$labState.opticsFocalLength} cm (focal length is considered negative mathematically for concave lens), object height (h_o) = ${ho} cm. Calculated image distance (d_i) = ${diStr}, linear magnification (m) = ${magnification}, image type = ${imageType}.`;
    } else if ($currentModule === 'Thermodynamics') {
      const T = $labState.gasTemperature;
      const V = $labState.gasVolume;
      const N = $labState.gasParticles;
      labContextInfo = `Current Experiment: Ideal Gas Kinetic Theory. Active parameters: gas temperature (T) = ${T} Kelvin, container volume factor (V) = ${V.toFixed(2)}, particle count (N) = ${N}. Ideal Gas Law relation states P * V = N * k * T.`;
    } else if ($currentModule === 'Electromagnetism') {
      const turns = $labState.solenoidTurns;
      const current = $labState.solenoidCurrent;
      const permeability = $labState.solenoidPermeability;
      const B = Math.abs(current) * turns * (permeability / 1000);
      labContextInfo = `Current Experiment: Solenoid Electromagnetism. Active parameters: number of turns (N) = ${turns}, electric current (I) = ${current.toFixed(1)} Amperes, relative permeability of core (mu_r) = ${permeability}, calculated magnetic flux index (B) = ${B.toFixed(2)}.`;
    }

    // Set correct system prompt based on active language
    const sysPromptLang = $currentLanguage === 'FR' 
      ? `Vous êtes un tuteur de laboratoire virtuel STEM localisé. Répondez directement et uniquement en français. Pas de blocs de pensée ou de balises <think>. Détaillez clairement les dérivations mathématiques étape par étape. Informations sur le laboratoire actuel: ${labContextInfo}`
      : $currentLanguage === 'SW'
      ? `Wewe ni Mwalimu wa Maabara ya STEM ya Ndani. Jibu moja kwa moja na kwa Kiswahili pekee. Usitoe mawazo au lebo za <think>. Eleza hatua kwa hatua makadirio yote ya hesabu. Maelezo ya sasa ya maabara: ${labContextInfo}`
      : `You are a Localized STEM Virtual Lab Tutor. Direct answer only in English. Do not output thinking blocks or <think> tags. Detail mathematical derivations step-by-step cleanly. Current virtual lab context: ${labContextInfo}`;

    // Package prompt with system prompt using ChatML formatting
    const formattedPrompt = `<|im_start|>system\n${sysPromptLang}<|im_end|>\n<|im_start|>user\n${textToSend}<|im_end|>\n<|im_start|>assistant\n`;

    try {
      await invoke('stream_stem_tutor_inference', { studentPrompt: formattedPrompt });
      isGenerating = false;
    } catch (err) {
      console.error("Inference Error:", err);
      isGenerating = false;
    }
  }
</script>

<!-- Top Navigation Bar -->
<header class="h-14 bg-white border-b border-slate-200 px-6 flex items-center justify-between">
  <div class="flex items-center gap-8">
    <div class="flex items-center gap-2">
      <img src="/nexus.png" alt="Nexa Lab Logo" class="w-7 h-7 object-contain rounded-lg" />
      <h1 class="text-xl font-bold tracking-wider text-blue-700 uppercase">NEXA LAB</h1>
    </div>
    <nav class="flex gap-6 font-medium text-sm">
      {#each ['Mechanics', 'Optics', 'Thermodynamics', 'Electromagnetism'] as mod}
        <button 
          class="pb-1 transition-all {$currentModule === mod ? 'text-blue-600 border-b-2 border-blue-600 font-semibold' : 'text-slate-500 hover:text-slate-800'}"
          on:click={() => $currentModule = mod}
        >
          {mod}
        </button>
      {/each}
    </nav>
  </div>

  <div class="flex items-center gap-4 text-xs font-semibold">
    <div class="flex border border-slate-200 rounded p-0.5 bg-slate-50">
      {#each ['EN', 'FR', 'SW'] as lang}
        <button 
          class="px-2 py-0.5 rounded transition-all {$currentLanguage === lang ? 'bg-blue-600 text-white' : 'text-slate-600'}"
          on:click={() => $currentLanguage = lang}
        >
          {lang}
        </button>
      {/each}
    </div>
    <button class="p-1.5 text-slate-500 hover:text-slate-700">🌐</button>
    <button class="p-1.5 text-slate-500 hover:text-slate-700">⚙️</button>
    <button class="p-1.5 text-slate-500 hover:text-slate-700">👤</button>
  </div>
</header>

<!-- Main Workbench Body -->
<div class="flex h-[calc(100vh-3.5rem)] bg-slate-100 p-4 gap-4">

  <!-- Sidebar -->
  <aside class="w-56 bg-white border border-slate-200 rounded-xl p-4 flex flex-col justify-between">
    <div>
      <div class="mb-6">
        <h2 class="text-base font-bold text-slate-900">Lab Explorer</h2>
        <p class="text-xs text-slate-500">Module: {$currentModule}</p>
      </div>

      <nav class="space-y-1 text-sm font-medium">
        <button class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-slate-600 hover:bg-slate-50" on:click={() => $isModuleModalOpen = true}>
          🧪 <span>Modules</span>
        </button>
        <button class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-blue-600 bg-blue-50 font-semibold">
          ⚗️ <span>Experiments</span>
        </button>
        <button class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-slate-600 hover:bg-slate-50">
          💾 <span>Saved States</span>
        </button>
        <button class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-slate-600 hover:bg-slate-50">
          📊 <span>Data Logs</span>
        </button>
        <button class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-slate-600 hover:bg-slate-50">
          📖 <span>Resources</span>
        </button>
      </nav>
    </div>

    <div class="space-y-3">
      <button 
        class="w-full bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2.5 px-4 rounded-lg text-sm transition-all shadow-sm flex items-center justify-center gap-2"
        on:click={() => $isHardwareModalOpen = true}
      >
        + NEW EXPERIMENT
      </button>
      <div class="pt-2 border-t border-slate-100 text-xs text-slate-500 space-y-1">
        <button class="block hover:underline">❓ Help</button>
        <button class="block hover:underline">📄 Docs</button>
      </div>
    </div>
  </aside>

  <!-- Center Panel: Interactive Canvas & Parameters -->
  <main class="flex-1 flex flex-col gap-4 overflow-hidden">
    <!-- Simulation Stage -->
    <div class="relative flex-1 bg-slate-900 rounded-xl overflow-hidden border border-slate-800 flex items-center justify-center">
      {#if $labState.experiment === 'simple_pendulum'}
        <div class="absolute top-4 left-4 bg-slate-800/80 backdrop-blur border border-slate-700 px-3 py-1 rounded text-xs text-slate-300 font-mono tracking-wider z-10">
          ● PENDULUM SIMULATION
        </div>
        <PendulumSim />
        <!-- Period Display Box -->
        <div class="absolute bottom-4 right-4 bg-slate-800/90 border border-slate-700 px-4 py-2 rounded-lg text-right z-10">
          <span class="block text-[10px] text-slate-400 font-mono">PERIOD (T)</span>
          <span class="text-lg font-mono font-bold text-cyan-400">{$labState.period} s</span>
        </div>
      {:else if $labState.experiment === 'solenoid'}
        <div class="absolute top-4 left-4 bg-slate-800/80 backdrop-blur border border-slate-700 px-3 py-1 rounded text-xs text-slate-300 font-mono tracking-wider z-10">
          ● SOLENOID ELECTROMAGNETISM
        </div>
        <SolenoidSim />
      {:else if $labState.experiment === 'ideal_gas'}
        <div class="absolute top-4 left-4 bg-slate-800/80 backdrop-blur border border-slate-700 px-3 py-1 rounded text-xs text-slate-300 font-mono tracking-wider z-10">
          ● KINETIC THEORY OF GASES
        </div>
        <GasSim />
      {:else if $labState.experiment === 'optics_lens'}
        <div class="absolute top-4 left-4 bg-slate-800/80 backdrop-blur border border-slate-700 px-3 py-1 rounded text-xs text-slate-300 font-mono tracking-wider z-10">
          ● THIN LENS OPTICS
        </div>
        <OpticsSim />
      {/if}
    </div>

    <!-- Parameter Dashboard -->
    <div class="bg-white border border-slate-200 rounded-xl p-4 flex gap-8">
      {#if $currentModule === 'Mechanics'}
        <!-- Mechanics (Simple Pendulum) -->
        <div class="flex-1 space-y-4">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Primary Parameters</h3>
          
          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Length (L)</span>
              <span class="text-blue-600 font-mono">{$labState.length.toFixed(2)} m</span>
            </div>
            <input type="range" min="0.1" max="2.0" step="0.05" bind:value={$labState.length} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Initial Angle (θ)</span>
              <span class="text-blue-600 font-mono">{$labState.angle}°</span>
            </div>
            <input type="range" min="5" max="90" step="1" bind:value={$labState.angle} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Mass (m)</span>
              <span class="text-blue-600 font-mono">{$labState.mass.toFixed(2)} kg</span>
            </div>
            <input type="range" min="0.1" max="5.0" step="0.1" bind:value={$labState.mass} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>
        </div>

        <div class="w-64 space-y-4 border-l border-slate-100 pl-8">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Environment Settings</h3>
          <div>
            <label for="damping-factor" class="block text-xs font-medium text-slate-600 mb-1">Damping Factor</label>
            <select id="damping-factor" bind:value={$labState.damping} class="w-full text-xs bg-slate-50 border border-slate-200 rounded-lg p-2 font-medium">
              <option>None</option>
              <option>Low (Air Friction)</option>
              <option>High (Viscous Fluid)</option>
            </select>
          </div>

          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Local Gravity (g)</span>
              <span class="text-blue-600 font-mono">{$labState.gravity.toFixed(2)} m/s²</span>
            </div>
            <input type="range" min="1.0" max="25.0" step="0.1" bind:value={$labState.gravity} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>
        </div>

      {:else if $currentModule === 'Optics'}
        <!-- Optics (Thin Lens) -->
        <div class="flex-1 space-y-4">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Optics Parameters</h3>
          
          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Object Distance (d_o)</span>
              <span class="text-blue-600 font-mono">{$labState.opticsObjectDistance} cm</span>
            </div>
            <input type="range" min="10" max="80" step="1" bind:value={$labState.opticsObjectDistance} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Focal Length (f)</span>
              <span class="text-blue-600 font-mono">{$labState.opticsFocalLength} cm</span>
            </div>
            <input type="range" min="5" max="40" step="1" bind:value={$labState.opticsFocalLength} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <span class="block text-xs font-semibold text-slate-600 mb-1">Lens Type</span>
            <div class="flex gap-2 bg-slate-50 border border-slate-200 p-0.5 rounded-lg">
              {#each ['Convex', 'Concave'] as type}
                <button 
                  class="flex-1 py-1 text-center rounded-md font-semibold text-xs transition-all { $labState.opticsLensType === type ? 'bg-blue-600 text-white shadow-sm font-bold' : 'text-slate-500 hover:text-slate-800' }"
                  on:click={() => $labState.opticsLensType = type}
                >
                  {type}
                </button>
              {/each}
            </div>
          </div>
        </div>

        <div class="w-64 space-y-4 border-l border-slate-100 pl-8">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Object Details</h3>
          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Object Height (h_o)</span>
              <span class="text-blue-600 font-mono">{$labState.opticsObjectHeight} cm</span>
            </div>
            <input type="range" min="5" max="30" step="1" bind:value={$labState.opticsObjectHeight} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div class="bg-slate-50 border border-slate-200 p-2 rounded-lg space-y-1 text-xs">
            <div class="flex justify-between">
              <span class="text-slate-500">Image Dist (d_i):</span>
              <span class="font-mono font-bold text-slate-800">
                {#if $labState.opticsLensType === 'Concave'}
                  {((- $labState.opticsFocalLength * $labState.opticsObjectDistance) / ($labState.opticsObjectDistance + $labState.opticsFocalLength)).toFixed(1)} cm
                {:else if Math.abs($labState.opticsObjectDistance - $labState.opticsFocalLength) < 0.05}
                  Infinity
                {:else}
                  {((($labState.opticsFocalLength * $labState.opticsObjectDistance) / ($labState.opticsObjectDistance - $labState.opticsFocalLength))).toFixed(1)} cm
                {/if}
              </span>
            </div>
            <div class="flex justify-between">
              <span class="text-slate-500">Image Type:</span>
              <span class="font-mono font-bold text-slate-800">
                {#if $labState.opticsLensType === 'Concave'}
                  Virtual (Upright, Diminished)
                {:else if Math.abs($labState.opticsObjectDistance - $labState.opticsFocalLength) < 0.05}
                  No Image
                {:else if (($labState.opticsFocalLength * $labState.opticsObjectDistance) / ($labState.opticsObjectDistance - $labState.opticsFocalLength)) > 0}
                  Real (Inverted)
                {:else}
                  Virtual (Upright, Magnified)
                {/if}
              </span>
            </div>
          </div>
        </div>

      {:else if $currentModule === 'Thermodynamics'}
        <!-- Thermodynamics (Ideal Gas) -->
        <div class="flex-1 space-y-4">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Gas Parameters</h3>
          
          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Temperature (T)</span>
              <span class="text-blue-600 font-mono">{$labState.gasTemperature} K</span>
            </div>
            <input type="range" min="100" max="600" step="10" bind:value={$labState.gasTemperature} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Volume (V)</span>
              <span class="text-blue-600 font-mono">{$labState.gasVolume.toFixed(2)} (Relative)</span>
            </div>
            <input type="range" min="0.50" max="1.50" step="0.05" bind:value={$labState.gasVolume} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>
        </div>

        <div class="w-64 space-y-4 border-l border-slate-100 pl-8">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Gas Composition</h3>
          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Particle Count (N)</span>
              <span class="text-blue-600 font-mono">{$labState.gasParticles}</span>
            </div>
            <input type="range" min="10" max="150" step="5" bind:value={$labState.gasParticles} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div class="bg-slate-50 border border-slate-200 p-2.5 rounded-lg flex items-center justify-between text-xs">
            <span class="text-slate-500">Gas Relation (PV/T):</span>
            <span class="font-mono font-bold text-slate-800">N · k_B</span>
          </div>
        </div>

      {:else if $currentModule === 'Electromagnetism'}
        <!-- Electromagnetism (Solenoid) -->
        <div class="flex-1 space-y-4">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Coil Parameters</h3>
          
          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Coil Turns (N)</span>
              <span class="text-blue-600 font-mono">{$labState.solenoidTurns} turns</span>
            </div>
            <input type="range" min="5" max="50" step="1" bind:value={$labState.solenoidTurns} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Electric Current (I)</span>
              <span class="text-blue-600 font-mono">{$labState.solenoidCurrent.toFixed(1)} A</span>
            </div>
            <input type="range" min="-5.0" max="5.0" step="0.1" bind:value={$labState.solenoidCurrent} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>
        </div>

        <div class="w-64 space-y-4 border-l border-slate-100 pl-8">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Magnetic Core</h3>
          <div>
            <div class="flex justify-between text-xs font-medium mb-1">
              <span>Relative Permeability (μ_r)</span>
              <span class="text-blue-600 font-mono">{$labState.solenoidPermeability}</span>
            </div>
            <input type="range" min="1" max="1000" step="10" bind:value={$labState.solenoidPermeability} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div class="bg-slate-50 border border-slate-200 p-2.5 rounded-lg flex items-center justify-between text-xs">
            <span class="text-slate-500">Flux Index B:</span>
            <span class="font-mono font-bold text-slate-800">
              {(Math.abs($labState.solenoidCurrent) * $labState.solenoidTurns * ($labState.solenoidPermeability / 1000)).toFixed(2)}
            </span>
          </div>
        </div>
      {/if}
    </div>
  </main>

  <!-- Right Panel: Nexa AI Tutor -->
  <aside class="w-96 bg-white border border-slate-200 rounded-xl flex flex-col justify-between overflow-hidden">
    <div class="p-3.5 border-b border-slate-200 flex items-center justify-between bg-slate-50">
      <div class="flex items-center gap-2.5">
        <img src="/nexus.png" alt="Nexa Lab Logo" class="w-7 h-7 object-contain rounded-lg" />
        <div>
          <h3 class="text-xs font-bold text-slate-900">Nexa STEM Tutor</h3>
          <p class="text-[10px] text-blue-600 font-mono">● {isGenerating ? 'GENERATING...' : 'ONLINE'}</p>
        </div>
      </div>
      <button class="text-slate-400 hover:text-slate-600">⋮</button>
    </div>

    <div class="px-4 py-2 border-b border-slate-200 bg-slate-50 flex gap-2">
      <button 
        class="flex-1 py-1.5 px-3 rounded-lg text-[11px] font-semibold bg-blue-600 hover:bg-blue-700 text-white shadow-sm flex items-center justify-center gap-1.5 transition-all"
        on:click={() => sendPrompt(`Start the ${$currentModule} virtual lab experiment. Please explain the objectives of the experiment, explain the physics theories and equations involved, and guide me on what parameters I can adjust and what physical effects I should observe.`)}
      >
        🚀 Start Experiment
      </button>
      <button 
        class="flex-1 py-1.5 px-3 rounded-lg text-[11px] font-semibold bg-emerald-600 hover:bg-emerald-700 text-white shadow-sm flex items-center justify-center gap-1.5 transition-all"
        on:click={() => sendPrompt(`Help me write a professional and structured laboratory report for the ${$currentModule} experiment based on the current active parameters. Please include:
1. Title
2. Objectives
3. Background & Theoretical Physics Principles (including LaTeX equations)
4. Experimental Setup & Simulation parameters
5. Observations & Data Analysis (using my active numbers)
6. Calculations & Discussion
7. Summary & Conclusion`)}
      >
        📝 Write Lab Report
      </button>
    </div>

    <!-- Chat Messages Stream with KaTeX Integration -->
    <div class="flex-1 p-4 overflow-y-auto space-y-3">
      {#each $chatMessages as msg}
        <div class="flex flex-col {msg.sender === 'user' ? 'items-end' : 'items-start'}">
          <div class="max-w-[85%] rounded-xl p-3 text-xs leading-relaxed {msg.sender === 'user' ? 'bg-blue-600 text-white' : 'bg-slate-100 text-slate-800 border border-slate-200'}">
            {#if msg.sender === 'user'}
              {msg.text}
            {:else}
              <KaTeX text={msg.text} />
            {/if}
          </div>
          <span class="text-[9px] text-slate-400 mt-1">{msg.time}</span>
        </div>
      {/each}
    </div>

    <!-- Action Pills & Input Bar -->
    <div class="p-3 border-t border-slate-200 bg-slate-50 space-y-2">
      <div class="flex gap-1.5 flex-wrap">
        {#if $currentModule === 'Mechanics'}
          <button 
            class="text-[10px] font-semibold bg-white border border-blue-300 text-blue-600 hover:bg-blue-50 px-2.5 py-1 rounded-full transition-all"
            on:click={() => sendPrompt("Derive the simple pendulum period formula step-by-step.")}
          >
            DERIVE FORMULA STEP-BY-STEP
          </button>
          <button 
            class="text-[10px] font-semibold bg-white border border-slate-300 text-slate-600 hover:bg-slate-100 px-2.5 py-1 rounded-full transition-all"
            on:click={() => sendPrompt("Explain why angle size affects the pendulum approximation.")}
          >
            EXPLAIN IN SIMPLER TERMS
          </button>
        {:else if $currentModule === 'Optics'}
          <button 
            class="text-[10px] font-semibold bg-white border border-blue-300 text-blue-600 hover:bg-blue-50 px-2.5 py-1 rounded-full transition-all"
            on:click={() => sendPrompt("Derive the lens maker and thin lens equation step-by-step.")}
          >
            DERIVE FORMULA STEP-BY-STEP
          </button>
          <button 
            class="text-[10px] font-semibold bg-white border border-slate-300 text-slate-600 hover:bg-slate-100 px-2.5 py-1 rounded-full transition-all"
            on:click={() => sendPrompt("Explain the difference between real and virtual images.")}
          >
            EXPLAIN IN SIMPLER TERMS
          </button>
        {:else if $currentModule === 'Thermodynamics'}
          <button 
            class="text-[10px] font-semibold bg-white border border-blue-300 text-blue-600 hover:bg-blue-50 px-2.5 py-1 rounded-full transition-all"
            on:click={() => sendPrompt("Derive the kinetic theory formula for gas pressure step-by-step.")}
          >
            DERIVE FORMULA STEP-BY-STEP
          </button>
          <button 
            class="text-[10px] font-semibold bg-white border border-slate-300 text-slate-600 hover:bg-slate-100 px-2.5 py-1 rounded-full transition-all"
            on:click={() => sendPrompt("Explain the relationship between temperature and molecular speed.")}
          >
            EXPLAIN IN SIMPLER TERMS
          </button>
        {:else if $currentModule === 'Electromagnetism'}
          <button 
            class="text-[10px] font-semibold bg-white border border-blue-300 text-blue-600 hover:bg-blue-50 px-2.5 py-1 rounded-full transition-all"
            on:click={() => sendPrompt("Derive the formula for magnetic field inside a solenoid step-by-step.")}
          >
            DERIVE FORMULA STEP-BY-STEP
          </button>
          <button 
            class="text-[10px] font-semibold bg-white border border-slate-300 text-slate-600 hover:bg-slate-100 px-2.5 py-1 rounded-full transition-all"
            on:click={() => sendPrompt("Explain how current direction and permeability affect polarity.")}
          >
            EXPLAIN IN SIMPLER TERMS
          </button>
        {/if}
      </div>

      <form class="flex items-center gap-2" on:submit|preventDefault={() => sendPrompt()}>
        <input 
          type="text" 
          placeholder="Ask Nexa about {$currentModule.toLowerCase()}..." 
          bind:value={userInput}
          class="flex-1 bg-white border border-slate-200 rounded-lg px-3 py-2 text-xs focus:outline-none focus:border-blue-500"
        />
        <button type="submit" class="bg-blue-600 hover:bg-blue-700 text-white p-2 rounded-lg transition-all text-xs">
          ➤
        </button>
      </form>
    </div>
  </aside>
</div>

<!-- Modals -->
<ModuleModal />
<HardwareModal />