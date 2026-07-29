<script>
  import { currentModule, isModuleModalOpen } from '$lib/stores/labStore';

  const modules = [
    {
      id: 'Mechanics',
      icon: '⚙️',
      title: 'Mechanics',
      desc: 'Kinematics, Dynamics, & Statics',
      capability: 'Provides advanced simulation tools for Newtonian physics, variable friction, projectile analysis, and rigid body dynamics.'
    },
    {
      id: 'Optics',
      icon: '👁️',
      title: 'Optics',
      desc: 'Lenses, Reflection, & Wave Theory',
      capability: 'Ray tracing, focal length calculations, wave interference patterns, and refractive index simulations.'
    },
    {
      id: 'Thermodynamics',
      icon: '🌡️',
      title: 'Thermodynamics',
      desc: 'Heat Exchange & Entropy Cycles',
      capability: 'Ideal gas law visualization, Carnot engines, heat transfer equations, and phase transitions.'
    },
    {
      id: 'Electromagnetism',
      icon: '⚡',
      title: 'Electromagnetism',
      desc: 'Circuits, Fields, & Induction',
      capability: 'Solenoid magnetic field lines, Faraday law, Lorentz forces, and RLC circuit responses.'
    },
    {
      id: 'Chemistry',
      icon: '🧪',
      title: 'Chemistry',
      desc: 'Titration, Solutions, & pH',
      capability: 'Real-time pH titration curves, indicator color transitions, and acid-base equivalence points.'
    }
  ];

  let selected = $currentModule;

  function confirmSelection() {
    $currentModule = selected;
    $isModuleModalOpen = false;
  }
</script>

{#if $isModuleModalOpen}
<div class="fixed inset-0 bg-slate-900/60 backdrop-blur-sm z-50 flex items-center justify-center p-4">
  <div class="bg-white border border-slate-200 rounded-2xl max-w-2xl w-full shadow-2xl overflow-hidden">
    
    <!-- Header -->
    <div class="p-6 text-center border-b border-slate-100 relative">
      <span class="text-[10px] font-mono tracking-widest text-blue-600 font-bold uppercase">Digital Workbench</span>
      <h2 class="text-lg font-bold text-slate-800">Select New Module</h2>
      <button 
        class="absolute top-4 right-4 text-slate-400 hover:text-slate-600 text-lg p-2"
        on:click={() => $isModuleModalOpen = false}
      >
        ✕
      </button>
    </div>

    <!-- Cards Grid -->
    <div class="p-6 grid grid-cols-5 gap-3">
      {#each modules as m}
        <button 
          class="flex flex-col items-center p-4 rounded-xl border text-center transition-all relative {selected === m.id ? 'border-blue-600 bg-blue-50/50 shadow-sm ring-2 ring-blue-600/20' : 'border-slate-200 hover:border-slate-300'}"
          on:click={() => selected = m.id}
        >
          {#if selected === m.id}
            <span class="absolute top-2 right-2 text-blue-600 text-xs font-bold">✓</span>
          {/if}
          <div class="w-10 h-10 rounded-lg bg-slate-100 flex items-center justify-center text-xl mb-3">
            {m.icon}
          </div>
          <h3 class="text-xs font-bold text-slate-800 mb-1">{m.title}</h3>
          <p class="text-[10px] text-slate-500 leading-tight">{m.desc}</p>
        </button>
      {/each}
    </div>

    <!-- Capability Banner -->
    <div class="px-6 py-4 bg-slate-50 border-y border-slate-100 mx-6 rounded-xl flex gap-3 items-start">
      <div class="w-6 h-6 rounded-full bg-blue-100 text-blue-600 flex items-center justify-center text-xs font-bold shrink-0">ℹ</div>
      <div>
        <span class="block text-[10px] font-mono text-blue-600 font-bold uppercase">MODULE CAPABILITY</span>
        <p class="text-xs text-slate-600 mt-0.5">
          {modules.find(m => m.id === selected)?.capability}
        </p>
      </div>
    </div>

    <!-- Action Bar -->
    <div class="p-6 flex items-center justify-between">
      <span class="text-[10px] font-mono text-slate-400">● READY FOR INITIALIZATION</span>
      <button 
        class="bg-blue-600 hover:bg-blue-700 text-white font-semibold px-6 py-2.5 rounded-lg text-xs transition-all shadow-sm"
        on:click={confirmSelection}
      >
        Confirm Selection →
      </button>
    </div>

  </div>
</div>
{/if}