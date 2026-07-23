<script>
  import { isHardwareModalOpen } from '$lib/stores/labStore';

  let currentStep = 1;
  let steps = [
    { title: "Physical Assembly", state: "done" },
    { title: "Sensor Calibration", state: "active" },
    { title: "System Test", state: "pending" }
  ];

  let checkList = [
    { id: 1, text: "Level the base plate.", checked: true },
    { id: 2, text: "Align laser sensor to zero-point.", checked: false },
    { id: 3, text: "Connect USB-C data cable.", checked: false }
  ];
</script>

{#if $isHardwareModalOpen}
<div class="fixed inset-0 bg-slate-900/60 backdrop-blur-sm z-50 flex items-center justify-center p-4">
  <div class="bg-white border border-slate-200 rounded-2xl max-w-2xl w-full shadow-2xl overflow-hidden">
    
    <!-- Header -->
    <div class="p-6 border-b border-slate-100 flex justify-between items-center">
      <h2 class="text-base font-bold text-slate-800">Hardware Setup: Pendulum Pivot</h2>
      <button class="text-slate-400 hover:text-slate-600" on:click={() => $isHardwareModalOpen = false}>✕</button>
    </div>

    <!-- Wizard Steps Stepper -->
    <div class="px-8 py-3 bg-slate-50 border-b border-slate-100 flex items-center justify-between text-xs font-semibold text-slate-500">
      <div class="flex items-center gap-2 text-blue-600">
        <span class="w-5 h-5 rounded-full bg-blue-600 text-white flex items-center justify-center text-[10px]">1</span>
        PHYSICAL ASSEMBLY
      </div>
      <div class="h-px bg-slate-200 w-12"></div>
      <div class="flex items-center gap-2 text-slate-400">
        <span class="w-5 h-5 rounded-full bg-slate-200 text-slate-600 flex items-center justify-center text-[10px]">2</span>
        SENSOR CALIBRATION
      </div>
      <div class="h-px bg-slate-200 w-12"></div>
      <div class="flex items-center gap-2 text-slate-400">
        <span class="w-5 h-5 rounded-full bg-slate-200 text-slate-600 flex items-center justify-center text-[10px]">3</span>
        SYSTEM TEST
      </div>
    </div>

    <!-- Body Layout -->
    <div class="p-6 grid grid-cols-2 gap-6 items-center">
      <!-- Diagram Blueprint Frame -->
      <div class="bg-slate-900 rounded-xl p-6 h-64 border border-slate-800 flex flex-col items-center justify-center relative overflow-hidden">
        <div class="absolute inset-0 bg-[radial-gradient(#334155_1px,transparent_1px)] [background-size:16px_16px] opacity-30"></div>
        <div class="w-32 h-32 border-2 border-dashed border-cyan-500/50 rounded-lg flex items-center justify-center text-cyan-400 text-xs font-mono">
          [ PIVOT MOUNT SCHEMATIC ]
        </div>
      </div>

      <!-- Checklist -->
      <div class="space-y-4">
        <div>
          <span class="text-[10px] font-mono text-blue-600 font-bold uppercase">STEP 1 OF 3</span>
          <h3 class="text-base font-bold text-slate-800">Secure Pivot Mount</h3>
        </div>

        <div class="space-y-2.5 text-xs text-slate-700 font-medium">
          {#each checkList as item}
            <label class="flex items-center gap-3 cursor-pointer">
              <input type="checkbox" bind:checked={item.checked} class="accent-blue-600 rounded w-4 h-4" />
              <span class={item.checked ? "line-through text-slate-400" : ""}>{item.text}</span>
            </label>
          {/each}
        </div>

        <div class="p-3 bg-blue-50 border border-blue-100 rounded-lg text-[11px] text-blue-700 leading-tight">
          💡 Ensure the pivot axis is perfectly perpendicular to the gravity vector for accurate period measurement.
        </div>
      </div>
    </div>

    <!-- Footer Buttons -->
    <div class="p-4 border-t border-slate-100 flex justify-end gap-3 bg-slate-50">
      <button 
        class="px-4 py-2 border border-slate-300 rounded-lg text-xs font-semibold text-slate-600 hover:bg-white"
        on:click={() => $isHardwareModalOpen = false}
      >
        CANCEL
      </button>
      <button 
        class="px-5 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-semibold shadow-sm"
        on:click={() => $isHardwareModalOpen = false}
      >
        INITIALIZE SENSOR
      </button>
    </div>

  </div>
</div>
{/if}