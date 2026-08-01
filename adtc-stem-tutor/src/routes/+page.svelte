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
    isHardwareModalOpen,
    titrationState,
    currentPH
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
  import TitrationStage from '$lib/components/simulations/TitrationSim.svelte';

  let userInput = "";
  let isGenerating = false;

  // Titration Simulation Controls & State
  let titrationSimInstance: any;
  let isFlowing = false;
  let flowRate = 1.0;
  let isChatOpen = true;

  let windowWidth = 1280;
  let windowHeight = 800;

  // Reactively close chat if window width gets too small
  let previousWidth = 1280;
  $: if (windowWidth < 1024 && previousWidth >= 1024) {
    isChatOpen = false;
  }
  $: previousWidth = windowWidth;

  function getIndicatorRangeText(ind: string) {
    if (ind === 'Phenolphthalein') return '8.2 - 9.8';
    if (ind === 'Methyl Orange') return '3.1 - 4.4';
    if (ind === 'Bromothymol Blue') return '6.0 - 7.6';
    return '';
  }
  
  function getIndicatorGradientStyle(ind: string) {
    if (ind === 'Phenolphthalein') {
      return 'background: linear-gradient(to right, rgba(226, 232, 240, 0.15) 0%, rgba(226, 232, 240, 0.15) 58%, rgba(244, 63, 94, 0.8) 70%, rgba(244, 63, 94, 0.8) 100%)';
    }
    if (ind === 'Methyl Orange') {
      return 'background: linear-gradient(to right, rgba(239, 68, 68, 0.8) 0%, rgba(239, 68, 68, 0.8) 22%, rgba(245, 158, 11, 0.8) 27%, rgba(234, 179, 8, 0.8) 32%, rgba(234, 179, 8, 0.8) 100%)';
    }
    if (ind === 'Bromothymol Blue') {
      return 'background: linear-gradient(to right, rgba(234, 179, 8, 0.8) 0%, rgba(234, 179, 8, 0.8) 42%, rgba(34, 197, 94, 0.8) 48%, rgba(59, 130, 246, 0.8) 54%, rgba(59, 130, 246, 0.8) 100%)';
    }
    return 'background: rgba(226, 232, 240, 0.1)';
  }

  function getIndicatorDescription(ind: string) {
    if (ind === 'Phenolphthalein') {
      return 'Phenolphthalein is colorless in acidic solution (pH < 8.2) and transitions to pink in basic solution (pH > 9.8).';
    }
    if (ind === 'Methyl Orange') {
      return 'Methyl Orange is red in strongly acidic solution (pH < 3.1) and transitions to yellow/orange in weaker acid (pH > 4.4).';
    }
    if (ind === 'Bromothymol Blue') {
      return 'Bromothymol Blue is yellow in acidic solution (pH < 6.0), transitions through green, and turns blue in basic solution (pH > 7.6).';
    }
    return '';
  }

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
    } else if ($currentModule === 'Chemistry') {
      $labState.experiment = 'titration';
    }
  }

  // Reactively calculate period when L or g changes for pendulum
  $: {
    const L = $labState.length;
    const g = $labState.gravity;
    $labState.period = parseFloat((2 * Math.PI * Math.sqrt(L / g)).toFixed(2));
  }

  // Chemistry titration pH is imported as $currentPH from labStore.js

  // Handle stream from Tauri backend
  onMount(() => {
    if (window.innerWidth < 1280) {
      isChatOpen = false;
    }

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
    } else if ($currentModule === 'Chemistry') {
      labContextInfo = `Current Experiment: Acid-Base Titration. Active parameters: titrant = ${$titrationState.titrant}, analyte = ${$titrationState.analyte}, titrant concentration = ${$titrationState.titrantConc} M, analyte volume = ${$titrationState.analyteVolume} mL, current pH = ${$currentPH}, indicator = ${$titrationState.indicator}.`;
    }

    // Set correct system prompt based on active language
    const sysPromptLang = $currentLanguage === 'FR' 
      ? `Vous êtes un tuteur de laboratoire virtuel STEM localisé. Répondez directement et uniquement en français. Pas de blocs de pensée ou de balises <think>. Détaillez clairement les dérivations mathématiques étape par étape. Informations sur le laboratoire actuel: ${labContextInfo}`
      : $currentLanguage === 'SW'
      ? `Wewe ni Mwalimu wa Maabara ya STEM ya Ndani. Jibu moja kwa moja na kwa Kiswahili pekee. Usitoe mawazo au lebo za <think>. Eleza hatua kwa hatua makadirio yote ya hesabu. Maelezo ya sasa ya maabara: ${labContextInfo}`
      : `You are a Localized STEM Virtual Lab Tutor. Direct answer only in English. Do not output thinking blocks or <think> tags. Detail mathematical derivations step-by-step cleanly. Current virtual lab context: ${labContextInfo}`;

    let studentPrompt = textToSend;
    let userText = textToSend;
    if ($currentModule === 'Chemistry') {
      const promptContext = `
[Nexa Context: Chemistry Module - Titration]
- Titrant: ${$titrationState.titrant}
- Analyte: ${$titrationState.analyte}
- Added Volume: ${$titrationState.addedVolume} mL / 50.0 mL
- Current pH: ${$currentPH}
- Indicator: ${$titrationState.indicator} (Color: ${$currentPH > 8.2 ? 'Pink' : 'Colorless'})
- Equivalence Point Reached: ${$titrationState.addedVolume === 25.0 ? 'YES' : 'NO'}

Student Question: ${studentPrompt}
`;
      userText = promptContext;
    }

    // Package prompt with system prompt using ChatML formatting
    const formattedPrompt = `<|im_start|>system\n${sysPromptLang}<|im_end|>\n<|im_start|>user\n${userText}<|im_end|>\n<|im_start|>assistant\n`;

    try {
      await invoke('stream_stem_tutor_inference', { studentPrompt: formattedPrompt });
      isGenerating = false;
    } catch (err) {
      console.error("Inference Error:", err);
      isGenerating = false;
    }
  }
</script>

<svelte:window bind:innerWidth={windowWidth} bind:innerHeight={windowHeight} />

<!-- Top Navigation Bar -->
<header class="h-14 bg-white border-b border-slate-200 px-4 md:px-6 flex items-center justify-between">
  <div class="flex items-center gap-3 lg:gap-8">
    <div class="flex items-center gap-2">
      <img src="/nexus.png" alt="Nexa Lab Logo" class="w-6 h-6 md:w-7 md:h-7 object-contain rounded-lg" />
      <h1 class="text-sm lg:text-base xl:text-xl font-bold tracking-wider text-blue-700 uppercase hidden sm:block">NEXA LAB</h1>
    </div>
    <nav class="flex gap-2 lg:gap-6 font-medium text-[11px] lg:text-sm">
      {#each ['Mechanics', 'Optics', 'Thermodynamics', 'Electromagnetism', 'Chemistry'] as mod}
        <button 
          class="pb-1 transition-all {$currentModule === mod ? 'text-blue-600 border-b-2 border-blue-600 font-semibold' : 'text-slate-500 hover:text-slate-800'}"
          on:click={() => $currentModule = mod}
        >
          {mod}
        </button>
      {/each}
    </nav>
  </div>

  <div class="flex items-center gap-2 lg:gap-4 text-xs font-semibold">
    <div class="flex border border-slate-200 rounded p-0.5 bg-slate-50">
      {#each ['EN', 'FR', 'SW'] as lang}
        <button 
          class="px-1.5 lg:px-2 py-0.5 rounded transition-all text-[10px] lg:text-xs {$currentLanguage === lang ? 'bg-blue-600 text-white' : 'text-slate-600'}"
          on:click={() => $currentLanguage = lang}
        >
          {lang}
        </button>
      {/each}
    </div>
    <button 
      class="flex items-center gap-1 px-2 py-1 lg:px-2.5 lg:py-1.5 rounded-lg border border-slate-200 bg-white hover:bg-slate-50 text-slate-700 hover:text-slate-900 transition-all font-semibold cursor-pointer shadow-sm active:scale-95 text-[10px] lg:text-xs"
      on:click={() => isChatOpen = !isChatOpen}
      title="Toggle Nexa STEM Tutor panel"
    >
      💬 <span class="hidden sm:inline">{isChatOpen ? 'Hide Chat' : 'Show Chat'}</span>
    </button>
    <button class="p-1 text-slate-500 hover:text-slate-700 hidden md:block">🌐</button>
    <button class="p-1 text-slate-500 hover:text-slate-700 hidden md:block">⚙️</button>
    <button class="p-1 text-slate-500 hover:text-slate-700 hidden md:block">👤</button>
  </div>
</header>

<!-- Main Workbench Body -->
<div class="flex h-[calc(100vh-3.5rem)] bg-slate-100 p-2 sm:p-4 gap-2 sm:gap-4 overflow-hidden">

  <!-- Sidebar -->
  <aside class="w-12 sm:w-16 xl:w-56 bg-white border border-slate-200 rounded-xl p-1.5 sm:p-3 xl:p-4 flex flex-col justify-between transition-all duration-300">
    <div>
      <div class="mb-6 hidden xl:block">
        <h2 class="text-base font-bold text-slate-900">Lab Explorer</h2>
        <p class="text-xs text-slate-500">Module: {$currentModule}</p>
      </div>
      <div class="mb-6 block xl:hidden text-center text-base sm:text-lg font-bold text-blue-600">
        🔬
      </div>

      <nav class="space-y-1 text-sm font-medium">
        <button class="w-full flex items-center justify-center xl:justify-start gap-3 px-1.5 sm:px-3 py-2 rounded-lg text-slate-600 hover:bg-slate-50 cursor-pointer" on:click={() => $isModuleModalOpen = true} title="Modules">
          🧪 <span class="hidden xl:inline">Modules</span>
        </button>
        <button class="w-full flex items-center justify-center xl:justify-start gap-3 px-1.5 sm:px-3 py-2 rounded-lg text-blue-600 bg-blue-50 font-semibold cursor-pointer" title="Experiments">
          ⚗️ <span class="hidden xl:inline">Experiments</span>
        </button>
        <button class="w-full flex items-center justify-center xl:justify-start gap-3 px-1.5 sm:px-3 py-2 rounded-lg text-slate-600 hover:bg-slate-50 cursor-pointer" title="Saved States">
          💾 <span class="hidden xl:inline">Saved States</span>
        </button>
        <button class="w-full flex items-center justify-center xl:justify-start gap-3 px-1.5 sm:px-3 py-2 rounded-lg text-slate-600 hover:bg-slate-50 cursor-pointer" title="Data Logs">
          📊 <span class="hidden xl:inline">Data Logs</span>
        </button>
        <button class="w-full flex items-center justify-center xl:justify-start gap-3 px-1.5 sm:px-3 py-2 rounded-lg text-slate-600 hover:bg-slate-50 cursor-pointer" title="Resources">
          📖 <span class="hidden xl:inline">Resources</span>
        </button>
      </nav>
    </div>

    <div class="space-y-3">
      <button 
        class="w-full bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2 px-1 sm:py-2.5 sm:px-2 xl:px-4 rounded-lg text-[10px] sm:text-xs xl:text-sm transition-all shadow-sm flex items-center justify-center gap-2 cursor-pointer"
        on:click={() => $isHardwareModalOpen = true}
        title="New Experiment"
      >
        <span>+</span> <span class="hidden xl:inline">NEW EXPERIMENT</span>
      </button>
      <div class="pt-2 border-t border-slate-100 text-xs text-slate-500 space-y-1 hidden xl:block text-left">
        <button class="block hover:underline">❓ Help</button>
        <button class="block hover:underline">📄 Docs</button>
      </div>
      <div class="pt-2 border-t border-slate-100 text-center text-slate-500 space-y-1 block xl:hidden">
        <button class="block hover:underline mx-auto text-sm sm:text-base" title="Help & Docs">❓</button>
      </div>
    </div>
  </aside>

  <!-- Center Panel: Interactive Canvas & Parameters -->
  <main class="flex-1 flex flex-col gap-2 sm:gap-4 overflow-hidden">
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
      {:else if $labState.experiment === 'titration'}
        <div class="absolute top-4 left-4 bg-slate-800/80 backdrop-blur border border-slate-700 px-3 py-1 rounded text-xs text-slate-300 font-mono tracking-wider z-10">
          ● ACID-BASE TITRATION
        </div>
        <TitrationStage bind:this={titrationSimInstance} bind:isFlowing={isFlowing} bind:flowRate={flowRate} />
      {/if}
    </div>

    <!-- Parameter Dashboard -->
    <div class="bg-white border border-slate-200 rounded-xl p-3 sm:p-4 flex flex-col xl:flex-row gap-4 xl:gap-8 overflow-y-auto xl:overflow-y-visible max-h-[35vh] xl:max-h-none shadow-sm">
      {#if $currentModule === 'Mechanics'}
        <!-- Mechanics (Simple Pendulum) -->
        <div class="flex-1 space-y-2 lg:space-y-4">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Primary Parameters</h3>
          
          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span>Length (L)</span>
              <span class="text-blue-600 font-mono">{$labState.length.toFixed(2)} m</span>
            </div>
            <input type="range" min="0.1" max="2.0" step="0.05" bind:value={$labState.length} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span>Initial Angle (θ)</span>
              <span class="text-blue-600 font-mono">{$labState.angle}°</span>
            </div>
            <input type="range" min="5" max="90" step="1" bind:value={$labState.angle} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span>Mass (m)</span>
              <span class="text-blue-600 font-mono">{$labState.mass.toFixed(2)} kg</span>
            </div>
            <input type="range" min="0.1" max="5.0" step="0.1" bind:value={$labState.mass} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>
        </div>

        <div class="w-full xl:w-64 space-y-2 lg:space-y-4 border-t xl:border-t-0 xl:border-l border-slate-100 pt-4 xl:pt-0 pl-0 xl:pl-8">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Environment Settings</h3>
          <div>
            <label for="damping-factor" class="block text-[11px] lg:text-xs font-medium text-slate-600 mb-1">Damping Factor</label>
            <select id="damping-factor" bind:value={$labState.damping} class="w-full text-xs bg-slate-50 border border-slate-200 rounded-lg p-2 font-medium">
              <option>None</option>
              <option>Low (Air Friction)</option>
              <option>High (Viscous Fluid)</option>
            </select>
          </div>

          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span>Local Gravity (g)</span>
              <span class="text-blue-600 font-mono">{$labState.gravity.toFixed(2)} m/s²</span>
            </div>
            <input type="range" min="1.0" max="25.0" step="0.1" bind:value={$labState.gravity} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>
        </div>

      {:else if $currentModule === 'Optics'}
        <!-- Optics (Thin Lens) -->
        <div class="flex-1 space-y-2 lg:space-y-4">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Optics Parameters</h3>
          
          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span>Object Distance (d_o)</span>
              <span class="text-blue-600 font-mono">{$labState.opticsObjectDistance} cm</span>
            </div>
            <input type="range" min="10" max="80" step="1" bind:value={$labState.opticsObjectDistance} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span>Focal Length (f)</span>
              <span class="text-blue-600 font-mono">{$labState.opticsFocalLength} cm</span>
            </div>
            <input type="range" min="5" max="40" step="1" bind:value={$labState.opticsFocalLength} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <span class="block text-[11px] lg:text-xs font-semibold text-slate-600 mb-1">Lens Type</span>
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

        <div class="w-full xl:w-64 space-y-2 lg:space-y-4 border-t xl:border-t-0 xl:border-l border-slate-100 pt-4 xl:pt-0 pl-0 xl:pl-8">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Object Details</h3>
          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
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
        <div class="flex-1 space-y-2 lg:space-y-4">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Gas Parameters</h3>
          
          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span>Temperature (T)</span>
              <span class="text-blue-600 font-mono">{$labState.gasTemperature} K</span>
            </div>
            <input type="range" min="100" max="600" step="10" bind:value={$labState.gasTemperature} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span>Volume (V)</span>
              <span class="text-blue-600 font-mono">{$labState.gasVolume.toFixed(2)} (Relative)</span>
            </div>
            <input type="range" min="0.50" max="1.50" step="0.05" bind:value={$labState.gasVolume} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>
        </div>

        <div class="w-full xl:w-64 space-y-2 lg:space-y-4 border-t xl:border-t-0 xl:border-l border-slate-100 pt-4 xl:pt-0 pl-0 xl:pl-8">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Gas Composition</h3>
          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
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
        <div class="flex-1 space-y-2 lg:space-y-4">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Coil Parameters</h3>
          
          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span>Coil Turns (N)</span>
              <span class="text-blue-600 font-mono">{$labState.solenoidTurns} turns</span>
            </div>
            <input type="range" min="5" max="50" step="1" bind:value={$labState.solenoidTurns} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span>Electric Current (I)</span>
              <span class="text-blue-600 font-mono">{$labState.solenoidCurrent.toFixed(1)} A</span>
            </div>
            <input type="range" min="-5.0" max="5.0" step="0.1" bind:value={$labState.solenoidCurrent} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>
        </div>

        <div class="w-full xl:w-64 space-y-2 lg:space-y-4 border-t xl:border-t-0 xl:border-l border-slate-100 pt-4 xl:pt-0 pl-0 xl:pl-8">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Magnetic Core</h3>
          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
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
      {:else if $currentModule === 'Chemistry'}
        <!-- Chemistry (Titration) -->
        <!-- Col 1: Titration Parameters -->
        <div class="flex-1 space-y-2 lg:space-y-4">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Titration Parameters</h3>
          
          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span class="text-slate-700">Titrant Concentration (M₁)</span>
              <span class="text-blue-600 font-mono">{$titrationState.titrantConc.toFixed(2)} M</span>
            </div>
            <input type="range" min="0.05" max="0.50" step="0.01" bind:value={$titrationState.titrantConc} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>

          <div>
            <div class="flex justify-between text-[11px] lg:text-xs font-medium mb-1">
              <span class="text-slate-700">Analyte Volume (V₂)</span>
              <span class="text-blue-600 font-mono">{$titrationState.analyteVolume.toFixed(1)} mL</span>
            </div>
            <input type="range" min="10.0" max="50.0" step="0.5" bind:value={$titrationState.analyteVolume} class="w-full accent-blue-600 h-1.5 bg-slate-200 rounded-lg cursor-pointer" />
          </div>
        </div>

        <!-- Col 2: Simulation Controls -->
        <div class="flex-1 border-t xl:border-t-0 xl:border-l border-slate-100 pt-4 xl:pt-0 pl-0 xl:pl-8 space-y-2 lg:space-y-3">
          <div class="flex justify-between items-center">
            <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Simulation Controls</h3>
            <div class="flex items-center gap-1">
              <span class="text-[9px] text-slate-400 font-semibold">Eq. Point:</span>
              <span class="text-[9px] font-mono font-bold text-rose-600 bg-rose-50 px-1.5 py-0.5 rounded border border-rose-200">
                {($titrationState.analyteVolume * ($titrationState.analyteConc / $titrationState.titrantConc)).toFixed(2)} mL
              </span>
            </div>
          </div>
          
          <!-- Flow Control buttons -->
          <div class="flex gap-2">
            {#if !isFlowing}
              <button 
                class="flex-1 bg-emerald-600 hover:bg-emerald-700 text-white font-semibold py-1.5 px-3 rounded text-xs transition-all flex items-center justify-center gap-1.5 shadow-sm border border-emerald-500 cursor-pointer active:scale-95 disabled:opacity-50 disabled:pointer-events-none"
                on:click={() => isFlowing = true}
                disabled={$titrationState.addedVolume >= 50.0}
              >
                <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 fill-current" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
                START
              </button>
            {:else}
              <button 
                class="flex-1 bg-amber-600 hover:bg-amber-600 text-white font-semibold py-1.5 px-3 rounded text-xs transition-all flex items-center justify-center gap-1.5 shadow-sm border border-amber-500 cursor-pointer active:scale-95"
                on:click={() => isFlowing = false}
              >
                <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 fill-current" viewBox="0 0 24 24"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>
                PAUSE
              </button>
            {/if}
            
            <button 
              class="bg-slate-100 hover:bg-slate-200 text-slate-700 font-semibold py-1.5 px-3 rounded text-xs transition-all flex items-center justify-center gap-1 shadow-sm border border-slate-300 cursor-pointer active:scale-95"
              on:click={() => titrationSimInstance?.resetTitration()}
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 fill-none stroke-current stroke-2" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 1121.21 7.89M9 11l3-3 3 3"/></svg>
              RESET
            </button>
          </div>

          <!-- Flow rate slider -->
          <div class="space-y-1">
            <div class="flex justify-between items-center text-[10px] font-medium">
              <span class="text-slate-500">Flow Speed:</span>
              <span class="font-mono font-bold text-blue-600">{flowRate.toFixed(2)} mL/s</span>
            </div>
            <input 
              type="range" 
              min="0.1" 
              max="3.0" 
              step="0.1" 
              bind:value={flowRate} 
              class="w-full h-1 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-blue-600" 
            />
          </div>

          <!-- Fine Drop Additions -->
          <div class="space-y-1">
            <span class="block text-[9px] font-bold text-slate-400 uppercase tracking-wider">Fine Drop Additions</span>
            <div class="grid grid-cols-4 gap-1.5">
              <button 
                class="bg-slate-50 hover:bg-slate-100 text-slate-800 py-1.5 px-1 rounded text-[10px] font-mono font-semibold transition-all border border-slate-200 cursor-pointer hover:border-blue-500 active:scale-95 disabled:opacity-50 disabled:pointer-events-none"
                on:click={() => titrationSimInstance?.triggerManualAddition(0.05)}
                disabled={$titrationState.addedVolume >= 50.0}
                title="Add 1 Drop (+0.05 mL)"
              >
                +0.05
              </button>
              <button 
                class="bg-slate-50 hover:bg-slate-100 text-slate-800 py-1.5 px-1 rounded text-[10px] font-mono font-semibold transition-all border border-slate-200 cursor-pointer hover:border-blue-500 active:scale-95 disabled:opacity-50 disabled:pointer-events-none"
                on:click={() => titrationSimInstance?.triggerManualAddition(0.20)}
                disabled={$titrationState.addedVolume >= 50.0}
                title="Add 4 Drops (+0.20 mL)"
              >
                +0.20
              </button>
              <button 
                class="bg-slate-50 hover:bg-slate-100 text-slate-800 py-1.5 px-1 rounded text-[10px] font-mono font-semibold transition-all border border-slate-200 cursor-pointer hover:border-blue-500 active:scale-95 disabled:opacity-50 disabled:pointer-events-none"
                on:click={() => titrationSimInstance?.triggerManualAddition(1.00)}
                disabled={$titrationState.addedVolume >= 50.0}
                title="Fast addition (+1.00 mL)"
              >
                +1.00
              </button>
              <button 
                class="bg-slate-50 hover:bg-slate-100 text-slate-800 py-1.5 px-1 rounded text-[10px] font-mono font-semibold transition-all border border-slate-200 cursor-pointer hover:border-blue-500 active:scale-95 disabled:opacity-50 disabled:pointer-events-none"
                on:click={() => titrationSimInstance?.triggerManualAddition(5.00)}
                disabled={$titrationState.addedVolume >= 50.0}
                title="Pour addition (+5.00 mL)"
              >
                +5.00
              </button>
            </div>
          </div>
        </div>

        <!-- Col 3: Chemical Settings & Indicator Spectrum -->
        <div class="w-full xl:w-80 border-t xl:border-t-0 xl:border-l border-slate-100 pt-4 xl:pt-0 pl-0 xl:pl-8 space-y-1.5 lg:space-y-2">
          <h3 class="text-xs font-bold text-slate-400 uppercase tracking-wider">Chemical Settings</h3>
          
          <div class="flex gap-4">
            <div class="flex-1">
              <label for="indicator-select" class="block text-[10px] font-medium text-slate-500 mb-0.5">Indicator Selection</label>
              <select id="indicator-select" bind:value={$titrationState.indicator} class="w-full text-xs bg-slate-50 border border-slate-200 rounded-lg p-1.5 font-medium focus:outline-none focus:border-blue-500">
                <option value="Phenolphthalein">Phenolphthalein</option>
                <option value="Methyl Orange">Methyl Orange</option>
                <option value="Bromothymol Blue">Bromothymol Blue</option>
              </select>
            </div>

            <div class="w-24 text-center">
              <span class="block text-[10px] font-medium text-slate-500 mb-0.5">Current pH</span>
              <div class="bg-slate-50 border border-slate-200 p-1.5 rounded-lg font-mono font-bold text-slate-800 text-xs">
                {$currentPH.toFixed(2)}
              </div>
            </div>
          </div>

          <!-- Indicator Spectrum -->
          <div class="space-y-1 relative pt-1">
            <div class="flex justify-between text-[8px] text-slate-400 font-semibold px-0.5">
              <span>pH 0</span>
              <span>{getIndicatorRangeText($titrationState.indicator)}</span>
              <span>pH 14</span>
            </div>
            
            <div 
              class="w-full h-3 rounded border border-slate-300 relative shadow-inner overflow-visible"
              style={getIndicatorGradientStyle($titrationState.indicator)}
            >
              <!-- Pointer at current pH -->
              <div 
                class="absolute -top-1.5 -bottom-1.5 w-1 bg-white border border-slate-900 shadow shadow-black transition-all duration-700 ease-out"
                style="left: calc({($currentPH / 14.0) * 100}% - 2px);"
              >
                <!-- Tooltip dot -->
                <div class="absolute -top-1.5 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-blue-500 border border-white"></div>
              </div>
            </div>
          </div>

          <!-- Description text -->
          <p class="text-[9px] text-slate-500 leading-normal line-clamp-2 mt-1">
            {getIndicatorDescription($titrationState.indicator)}
          </p>
        </div>
      {/if}
    </div>
  </main>

{#if isChatOpen}
  <!-- Right Panel: Nexa AI Tutor -->
  <aside class="w-72 lg:w-80 xl:w-96 bg-white border border-slate-200 rounded-xl flex flex-col justify-between overflow-hidden transition-all duration-300">
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
          {:else}
            {#if $currentModule === 'Optics'}
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
            {:else if $currentModule === 'Chemistry'}
              <button 
                class="text-[10px] font-semibold bg-white border border-blue-300 text-blue-600 hover:bg-blue-50 px-2.5 py-1 rounded-full transition-all"
                on:click={() => sendPrompt("Explain the equivalence point buffer equation")}
              >
                Explain the equivalence point buffer equation
              </button>
              <button 
                class="text-[10px] font-semibold bg-white border border-slate-300 text-slate-600 hover:bg-slate-100 px-2.5 py-1 rounded-full transition-all"
                on:click={() => sendPrompt("Derive $pH = -\\log_{10}[H^+]$ for this step")}
              >
                Derive $pH = -\log_{10}[H^+]$ for this step
              </button>
              <button 
                class="text-[10px] font-semibold bg-white border border-slate-300 text-slate-600 hover:bg-slate-100 px-2.5 py-1 rounded-full transition-all"
                on:click={() => sendPrompt("Why did the phenolphthalein color disappear?")}
              >
                Why did the phenolphthalein color disappear?
              </button>
            {/if}
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
  {/if}
</div>

<!-- Modals -->
<ModuleModal />
<HardwareModal />