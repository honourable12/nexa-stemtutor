<script lang="ts">
  import { samplingConfig } from '$lib/stores/labStore';

  let isOpen = false;

  export function open() { isOpen = true; }
  export function close() { isOpen = false; }

  // Persist to localStorage so user preferences survive restarts.
  // This is also pre-work for the v0.3 persistence feature.
  function persist() {
    try {
      localStorage.setItem('samplingConfig', JSON.stringify($samplingConfig));
    } catch (e) {
      console.warn('Could not persist sampling config:', e);
    }
  }

  try {
    const saved = localStorage.getItem('samplingConfig');
    if (saved) {
      const parsed = JSON.parse(saved);
      $samplingConfig = { ...$samplingConfig, ...parsed };
    }
  } catch (e) {
  
  }

  function resetDefaults() {
    $samplingConfig = {
      temperature: 0.7,
      top_p: 0.9,
      repeat_penalty: 1.1,
      max_tokens: 768
    };
    persist();
  }

  function update(field: string, value: number) {
    $samplingConfig = { ...$samplingConfig, [field]: value };
    persist();
  }
</script>

{#if isOpen}
  <div class="modal-overlay" on:click={close}>
    <div class="modal-card" on:click|stopPropagation>
      <div class="modal-header">
        <h2>Inference Settings</h2>
        <button class="close-btn" on:click={close} aria-label="Close">x</button>
      </div>

      <div class="modal-body">
        <div class="field">
          <label for="temp">
            Temperature: <span class="val">{$samplingConfig.temperature.toFixed(2)}</span>
          </label>
          <input
            id="temp"
            type="range" min="0" max="1.5" step="0.05"
            value={$samplingConfig.temperature}
            on:input={(e) => update('temperature', parseFloat(e.currentTarget.value))}
          />
          <p class="hint">Lower = more deterministic. 0 = greedy. Auto-forced to 0 for math prompts.</p>
        </div>

        <div class="field">
          <label for="topp">
            Top-p: <span class="val">{$samplingConfig.top_p.toFixed(2)}</span>
          </label>
          <input
            id="topp"
            type="range" min="0.5" max="1.0" step="0.01"
            value={$samplingConfig.top_p}
            on:input={(e) => update('top_p', parseFloat(e.currentTarget.value))}
          />
          <p class="hint">Nucleus sampling threshold. 1.0 = no filtering. (Reserved for v0.3 wiring.)</p>
        </div>

        <div class="field">
          <label for="penalty">
            Repeat Penalty: <span class="val">{$samplingConfig.repeat_penalty.toFixed(2)}</span>
          </label>
          <input
            id="penalty"
            type="range" min="1.0" max="1.3" step="0.01"
            value={$samplingConfig.repeat_penalty}
            on:input={(e) => update('repeat_penalty', parseFloat(e.currentTarget.value))}
          />
          <p class="hint">Penalizes repeated tokens. 1.0 = no penalty. (Reserved for v0.3 wiring.)</p>
        </div>

        <div class="field">
          <label for="maxt">Max Tokens (response length cap)</label>
          <input
            id="maxt"
            type="number" min="128" max="2048" step="64"
            value={$samplingConfig.max_tokens}
            on:input={(e) => update('max_tokens', parseInt(e.currentTarget.value) || 768)}
          />
          <p class="hint">Hard stop for response length. Prevents context overflow.</p>
        </div>
      </div>

      <div class="modal-footer">
        <button class="ghost-btn" on:click={resetDefaults}>Reset to defaults</button>
        <button class="primary-btn" on:click={close}>Done</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed; inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000;
  }
  .modal-card {
    background: #1a2332;
    border: 1px solid #3a5070;
    border-radius: 10px;
    width: 420px; max-width: 90vw;
    color: #e8f0f8;
    font-family: 'Inter', sans-serif;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  }
  .modal-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid #2d3f55;
  }
  .modal-header h2 {
    margin: 0; font-size: 16px; font-weight: 600;
    color: #4da8da;
  }
  .close-btn {
    background: transparent; border: none; color: #7a9bb8;
    cursor: pointer; font-size: 18px; padding: 0;
  }
  .close-btn:hover { color: #e8f0f8; }
  .modal-body { padding: 18px 20px; }
  .field { margin-bottom: 18px; }
  .field label {
    display: flex; justify-content: space-between;
    font-size: 13px; color: #b0c4d8;
    margin-bottom: 6px;
  }
  .val {
    color: #4da8da; font-family: 'DejaVu Sans Mono', monospace;
    font-weight: 600;
  }
  input[type="range"] {
    width: 100%; accent-color: #4da8da;
    cursor: pointer;
  }
  input[type="number"] {
    width: 100%; padding: 6px 10px;
    background: #0e1726; border: 1px solid #2d3f55;
    border-radius: 5px; color: #e8f0f8;
    font-family: 'DejaVu Sans Mono', monospace; font-size: 13px;
  }
  .hint {
    font-size: 11px; color: #5a7a96;
    margin: 4px 0 0 0;
  }
  .modal-footer {
    display: flex; justify-content: flex-end; gap: 10px;
    padding: 14px 20px;
    border-top: 1px solid #2d3f55;
  }
  .ghost-btn {
    background: transparent; border: 1px solid #3a5070;
    color: #b0c4d8; padding: 7px 14px;
    border-radius: 5px; cursor: pointer; font-size: 13px;
  }
  .ghost-btn:hover { background: #2d3f55; }
  .primary-btn {
    background: #2d7ab3; border: none; color: white;
    padding: 7px 18px; border-radius: 5px;
    cursor: pointer; font-size: 13px; font-weight: 500;
  }
  .primary-btn:hover { background: #3a8ac3; }
</style>
