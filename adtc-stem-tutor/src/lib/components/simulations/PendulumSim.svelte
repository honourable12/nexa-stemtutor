<script lang="ts">
  import { onMount } from 'svelte';
  import { labState } from '$lib/stores/labStore';

  let canvas: HTMLCanvasElement;
  let animationFrameId: number;
  let lastTime = 0;

  // Internal physics state
  let theta = ($labState.angle * Math.PI) / 180;
  let omega = 0;
  let trail: Array<{ x: number; y: number }> = [];

  // Reset when initial angle changes from the slider
  let prevAngle = $labState.angle;
  $: if ($labState.angle !== prevAngle) {
    theta = ($labState.angle * Math.PI) / 180;
    omega = 0;
    trail = [];
    prevAngle = $labState.angle;
  }

  // Clear trail if length or gravity changes
  let prevLength = $labState.length;
  $: if ($labState.length !== prevLength) {
    trail = [];
    prevLength = $labState.length;
  }

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
      // Cap dt to prevent huge steps when page is inactive
      if (dt > 0.05) dt = 0.05;
      lastTime = timestamp;

      // Physics integration (Euler-Cromer)
      const g = $labState.gravity;
      const L = $labState.length;
      let dampingCoeff = 0.0;
      if ($labState.damping === 'Low (Air Friction)') {
        dampingCoeff = 0.08;
      } else if ($labState.damping === 'High (Viscous Fluid)') {
        dampingCoeff = 0.6;
      }

      // angular acceleration alpha = -(g/L)*sin(theta) - damping*omega
      const alpha = -(g / L) * Math.sin(theta) - dampingCoeff * omega;
      omega += alpha * dt;
      theta += omega * dt;

      // Clear canvas
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const cx = canvas.width / 2;
      const cy = 50; // pivot y position
      const pixelsPerMeter = (canvas.height - 120) / 2.0;
      const lengthPx = L * pixelsPerMeter;

      const bobX = cx + lengthPx * Math.sin(theta);
      const bobY = cy + lengthPx * Math.cos(theta);

      // Save to trail
      trail.push({ x: bobX, y: bobY });
      if (trail.length > 50) {
        trail.shift();
      }

      // 1. Draw Angle Grid & Degrees Arc 
      ctx.strokeStyle = 'rgba(148, 163, 184, 0.15)';
      ctx.lineWidth = 1.0;
      ctx.setLineDash([4, 4]);

      // Vertical reference line
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(cx, canvas.height - 40);
      ctx.stroke();

      // Max angle guide lines
      const maxRad = ($labState.angle * Math.PI) / 180;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(cx + lengthPx * Math.sin(maxRad), cy + lengthPx * Math.cos(maxRad));
      ctx.moveTo(cx, cy);
      ctx.lineTo(cx - lengthPx * Math.sin(maxRad), cy + lengthPx * Math.cos(maxRad));
      ctx.stroke();
      ctx.setLineDash([]);

      // Angle indicator arc
      ctx.strokeStyle = 'rgba(148, 163, 184, 0.4)';
      ctx.beginPath();
      ctx.arc(cx, cy, 60, Math.PI/2 - Math.abs(theta), Math.PI/2 + Math.abs(theta));
      ctx.stroke();

      // 2. Draw Fading Motion Trail 
      if (trail.length > 1) {
        ctx.lineWidth = 2.0;
        for (let i = 0; i < trail.length - 1; i++) {
          const ratio = i / trail.length;
          ctx.strokeStyle = `rgba(34, 211, 238, ${ratio * 0.45})`;
          ctx.beginPath();
          ctx.moveTo(trail[i].x, trail[i].y);
          ctx.lineTo(trail[i + 1].x, trail[i + 1].y);
          ctx.stroke();
        }
      }

      // 3. Draw Pendulum String (Rod) 
      // Glowing rod effect
      ctx.save();
      ctx.shadowColor = '#06b6d4';
      ctx.shadowBlur = 4;
      ctx.strokeStyle = '#22d3ee';
      ctx.lineWidth = 3.0;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(bobX, bobY);
      ctx.stroke();
      ctx.restore();

      // Inner wire detail
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 1.0;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(bobX, bobY);
      ctx.stroke();

      // 4. Draw Pivot Mount 
      // Stand base
      ctx.fillStyle = '#475569';
      ctx.fillRect(cx - 20, cy - 8, 40, 8);
      // Pivot bolt
      ctx.fillStyle = '#94a3b8';
      ctx.beginPath();
      ctx.arc(cx, cy - 4, 6, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = '#334155';
      ctx.lineWidth = 1.5;
      ctx.stroke();

      // 5. Draw Bob
      ctx.save();
      ctx.shadowColor = '#22d3ee';
      ctx.shadowBlur = 12;
      
      const grad = ctx.createRadialGradient(bobX, bobY, 2, bobX, bobY, 16);
      grad.addColorStop(0, '#e0f7fa');
      grad.addColorStop(0.4, '#22d3ee');
      grad.addColorStop(1, '#0891b2');

      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(bobX, bobY, 16, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();

      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.arc(bobX, bobY, 16, 0, Math.PI * 2);
      ctx.stroke();

      // Bob mass text
      ctx.fillStyle = '#0f172a';
      ctx.font = 'bold 10px sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('m', bobX, bobY);

      // --- 6. Draw Dashboard Text on Canvas ---
      ctx.fillStyle = '#94a3b8';
      ctx.font = '11px monospace';
      ctx.textAlign = 'left';
      ctx.fillText(`ANGLE θ: ${(theta * 180 / Math.PI).toFixed(1)}°`, 20, 30);
      ctx.fillText(`ANGULAR VEL (ω): ${omega.toFixed(2)} rad/s`, 20, 48);

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
