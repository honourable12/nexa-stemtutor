<script lang="ts">
  import { onMount } from 'svelte';
  import { labState } from '$lib/stores/labStore';

  let canvas: HTMLCanvasElement;
  let animationFrameId: number;
  let offset = 0;
  let lastTime = 0;

  onMount(() => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const resize = () => {
      canvas.width = canvas.parentElement?.clientWidth || 400;
      canvas.height = canvas.parentElement?.clientHeight || 400;
    };
    window.addEventListener('resize', resize);
    resize();

    const loop = (timestamp: number) => {
      if (!lastTime) lastTime = timestamp;
      let dt = (timestamp - lastTime) / 1000;
      lastTime = timestamp;

      // Animate movement offset along field lines
      const current = $labState.solenoidCurrent;
      offset += current * 40 * dt; // speed depends on current strength

      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const cx = canvas.width / 2;
      const cy = canvas.height / 2;
      const coreW = 220;
      const coreH = 40;

      // Magnetic field strength B factor
      const B = Math.abs(current) * $labState.solenoidTurns * ($labState.solenoidPermeability / 1000);

      // 1. Draw Magnetic Field Lines 
      if (B > 0.05) {
        const permeabilityFactor = $labState.solenoidPermeability;
        ctx.lineWidth = 1.5;
        // Direction of current determines N-S polarity (arrow direction)
        const direction = current > 0 ? 1 : -1;

        // Number of lines drawn depends on field strength (B)
        const lineCount = Math.min(10, Math.max(2, Math.floor(B * 2) + 2));

        for (let i = 1; i <= lineCount; i++) {
          const rx = coreW * (0.8 + i * 0.25);
          const ry = coreH * (1.2 + i * 0.5);

          // Hue of magnetic flux changes based on permeability
          const glowColor = permeabilityFactor > 300 ? 'rgba(34, 211, 238,' : 'rgba(59, 130, 246,';
          ctx.strokeStyle = `${glowColor}${0.1 + (1 - i / lineCount) * 0.45})`;

          // Outer closed loops
          ctx.beginPath();
          ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2);
          ctx.stroke();

          // Particle indicators on lines to show direction of field
          ctx.fillStyle = permeabilityFactor > 300 ? '#22d3ee' : '#60a5fa';
          const particleCount = 4;
          for (let p = 0; p < particleCount; p++) {
            const angle = ((offset * direction / rx) + (p * (Math.PI * 2 / particleCount))) % (Math.PI * 2);
            const px = cx + rx * Math.cos(angle);
            const py = cy + ry * Math.sin(angle);

            ctx.beginPath();
            ctx.arc(px, py, 3, 0, Math.PI * 2);
            ctx.fill();
          }
        }

        // Draw internal field lines inside core (straight lines through center)
        ctx.strokeStyle = `${permeabilityFactor > 300 ? 'rgba(34, 211, 238, 0.6)' : 'rgba(96, 165, 250, 0.6)'}`;
        ctx.lineWidth = Math.min(4, 1 + B / 3);
        const internalLines = 3;
        for (let l = 0; l < internalLines; l++) {
          const ly = cy - coreH / 4 + (l * coreH / 8);
          ctx.beginPath();
          ctx.moveTo(cx - coreW / 2 - 20, ly);
          ctx.lineTo(cx + coreW / 2 + 20, ly);
          ctx.stroke();
        }
      }

      // 2. Draw Soft Iron Core Cylinder 
      // If permeability and current are high, core has a subtle magnetic glow
      if (B > 0.1 && $labState.solenoidPermeability > 200) {
        ctx.save();
        ctx.shadowColor = current > 0 ? '#06b6d4' : '#ef4444';
        ctx.shadowBlur = Math.min(25, $labState.solenoidPermeability / 20);
        ctx.fillStyle = '#475569';
        ctx.beginPath();
        ctx.roundRect(cx - coreW / 2, cy - coreH / 2, coreW, coreH, 4);
        ctx.fill();
        ctx.restore();
      } else {
        // Normal iron core
        ctx.fillStyle = '#475569';
        ctx.beginPath();
        ctx.roundRect(cx - coreW / 2, cy - coreH / 2, coreW, coreH, 4);
        ctx.fill();
      }

      // Cylinder end shading for 3D appearance
      ctx.fillStyle = '#334155';
      ctx.beginPath();
      ctx.ellipse(cx - coreW / 2, cy, 6, coreH / 2, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = '#64748b';
      ctx.beginPath();
      ctx.ellipse(cx + coreW / 2, cy, 6, coreH / 2, 0, 0, Math.PI * 2);
      ctx.fill();

      // 3. Draw Solenoid Copper Wire Turns
      const turns = $labState.solenoidTurns;
      const pitch = coreW / (turns + 1);
      ctx.strokeStyle = '#f59e0b'; // Amber copper color
      ctx.lineWidth = 4;
      ctx.lineCap = 'round';

      for (let t = 1; t <= turns; t++) {
        const tx = cx - coreW / 2 + t * pitch;

        // Front half of loop (visible, going over core)
        ctx.beginPath();
        ctx.arc(tx, cy, coreH / 2 + 4, -Math.PI / 2, Math.PI / 2, false);
        ctx.stroke();

        // Draw wire shadows/back connection details behind core (drawn as dashed or darker)
        ctx.strokeStyle = '#b45309';
        ctx.lineWidth = 2.5;
        ctx.beginPath();
        ctx.arc(tx - pitch / 2, cy, coreH / 2 + 3, Math.PI / 2, -Math.PI / 2, false);
        ctx.stroke();
        ctx.strokeStyle = '#f59e0b';
        ctx.lineWidth = 4;
      }

      // 4. Draw Polarity Markers (N / S) 
      if (Math.abs(current) > 0.01) {
        ctx.fillStyle = '#ffffff';
        ctx.font = 'bold 12px sans-serif';
        ctx.textAlign = 'center';

        const northLabel = current > 0 ? 'S' : 'N';
        const southLabel = current > 0 ? 'N' : 'S';

        ctx.fillText(northLabel, cx - coreW / 2 - 35, cy + 4);
        ctx.fillText(southLabel, cx + coreW / 2 + 35, cy + 4);
      }

      animationFrameId = requestAnimationFrame(loop);
    };

    animationFrameId = requestAnimationFrame(loop);

    return () => {
      window.removeEventListener('resize', resize);
      cancelAnimationFrame(animationFrameId);
    };
  });
</script>

<canvas bind:this={canvas} class="w-full h-full block bg-slate-950"></canvas>
