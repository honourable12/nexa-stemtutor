<script lang="ts">
  import { onMount } from 'svelte';
  import { labState } from '$lib/stores/labStore';

  let canvas: HTMLCanvasElement;
  let animationFrameId: number;
  let lastTime = 0;

  // Particle structure
  interface Particle {
    x: number;
    y: number;
    vx: number;
    vy: number;
    radius: number;
    color: string;
  }

  let particles: Particle[] = [];
  let pressureHistory: number[] = [];
  let displayPressure = 0;

  // Re-initialize particles when the count changes
  $: adjustParticleCount($labState.gasParticles);

  function adjustParticleCount(targetCount: number) {
    if (!canvas) return;
    const currentCount = particles.length;

    if (currentCount < targetCount) {
      // Add particles
      const V = $labState.gasVolume;
      const boxW = Math.min(canvas.width - 60, 320 * V);
      const boxH = 200;
      const xStart = (canvas.width - boxW) / 2;
      const yStart = (canvas.height - boxH) / 2;

      for (let i = currentCount; i < targetCount; i++) {
        particles.push({
          x: xStart + Math.random() * boxW,
          y: yStart + Math.random() * boxH,
          vx: (Math.random() - 0.5) * 100,
          vy: (Math.random() - 0.5) * 100,
          radius: 4,
          color: ''
        });
      }
    } else if (currentCount > targetCount) {
      // Remove particles
      particles = particles.slice(0, targetCount);
    }
  }

  onMount(() => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const resize = () => {
      canvas.width = canvas.parentElement?.clientWidth || 400;
      canvas.height = canvas.parentElement?.clientHeight || 400;
      adjustParticleCount($labState.gasParticles);
    };
    window.addEventListener('resize', resize);
    resize();

    const loop = (timestamp: number) => {
      if (!lastTime) lastTime = timestamp;
      let dt = (timestamp - lastTime) / 1000;
      if (dt > 0.05) dt = 0.05; // cap dt
      lastTime = timestamp;

      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const V = $labState.gasVolume;
      const T = $labState.gasTemperature;

      // Calculate Box Dimensions based on Volume
      const baseW = Math.min(canvas.width - 60, 320);
      const boxW = baseW * V;
      const boxH = 200;
      const xStart = (canvas.width - boxW) / 2;
      const yStart = (canvas.height - boxH) / 2;

      // Draw container box
      ctx.strokeStyle = '#ef4444'; // Red container
      ctx.lineWidth = 4;
      ctx.shadowColor = 'rgba(239, 68, 68, 0.4)';
      ctx.shadowBlur = 10;
      ctx.strokeRect(xStart, yStart, boxW, boxH);
      ctx.shadowBlur = 0; // reset

      // Target speed factor based on temperature (v = sqrt(T))
      const speedScale = Math.sqrt(T) * 12;

      let momentumExchange = 0;

      // Update & Draw Particles
      particles.forEach(p => {
        // Enforce velocity scale based on Temperature
        const currentSpeed = Math.sqrt(p.vx * p.vx + p.vy * p.vy);
        if (currentSpeed > 0.01) {
          p.vx = (p.vx / currentSpeed) * speedScale;
          p.vy = (p.vy / currentSpeed) * speedScale;
        } else {
          p.vx = (Math.random() - 0.5) * speedScale;
          p.vy = (Math.random() - 0.5) * speedScale;
        }

        // Move particle
        p.x += p.vx * dt;
        p.y += p.vy * dt;

        // Bounce off walls
        const r = p.radius;
        // Left & Right
        if (p.x - r < xStart) {
          p.x = xStart + r;
          p.vx = -p.vx;
          momentumExchange += 2 * Math.abs(p.vx);
        } else if (p.x + r > xStart + boxW) {
          p.x = xStart + boxW - r;
          p.vx = -p.vx;
          momentumExchange += 2 * Math.abs(p.vx);
        }
        // Top & Bottom
        if (p.y - r < yStart) {
          p.y = yStart + r;
          p.vy = -p.vy;
          momentumExchange += 2 * Math.abs(p.vy);
        } else if (p.y + r > yStart + boxH) {
          p.y = yStart + boxH - r;
          p.vy = -p.vy;
          momentumExchange += 2 * Math.abs(p.vy);
        }

        // Particle particle elastic collisions (optional simplified grid check)
        // Draw particle
        // Warm/cold color mapping
        const rVal = Math.min(255, Math.floor((T - 100) / 500 * 255));
        const bVal = Math.min(255, Math.floor((600 - T) / 500 * 255));
        ctx.fillStyle = `rgb(${rVal}, 100, ${bVal})`;

        ctx.beginPath();
        ctx.arc(p.x, p.y, p.radius, 0, Math.PI * 2);
        ctx.fill();
      });

      // Calculate dynamic wall pressure
      // Pressure P = F / A. Force F = dP/dt.
      // Area A is perimeter of 2D box = 2 * (boxW + boxH)
      const perimeter = 2 * (boxW + boxH);
      const instantPressure = (momentumExchange / dt) / perimeter * 0.05;

      pressureHistory.push(instantPressure);
      if (pressureHistory.length > 50) {
        pressureHistory.shift();
      }

      // Smooth pressure display
      const avgPressure = pressureHistory.reduce((a, b) => a + b, 0) / pressureHistory.length;
      displayPressure = avgPressure * 8 + (T * $labState.gasParticles) / (boxW * boxH) * 5; // combine theoretical fallback for stability

      // Draw Pressure Meter inside simulation box
      ctx.fillStyle = '#94a3b8';
      ctx.font = '11px monospace';
      ctx.textAlign = 'left';
      ctx.fillText(`CONTAINER PRESSURE: ${displayPressure.toFixed(1)} kPa`, xStart + 10, yStart - 10);
      ctx.fillText(`VOLUME: ${(V * 100).toFixed(0)} L`, xStart + 10, yStart + boxH + 20);

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
