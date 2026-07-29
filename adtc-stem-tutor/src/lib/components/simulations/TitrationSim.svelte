<script lang="ts">
  import { onMount } from 'svelte';
  import { titrationState, currentPH } from '$lib/stores/labStore';

  let canvas: HTMLCanvasElement;
  let animationFrameId: number;
  let lastTime = 0;
  
  // Animation and Flow state
  let isFlowing = false;
  let flowRate = 1.0; // mL per second
  let drops: Array<{ y: number; speed: number }> = [];
  let dropTimer = 0;

  // Local helper for pH calculation (needed for plotting the full curve)
  function calculatePH(addedVol: number, titrantConc: number, analyteVol: number, analyteConc: number) {
    const vAcid = analyteVol;
    const cAcid = analyteConc;
    const cBase = titrantConc;
    const vAdded = addedVol;
    
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
  }

  // Smooth RGB color interpolation for indicators based on pH
  function getIndicatorColor(indicator: string, ph: number) {
    if (indicator === 'Phenolphthalein') {
      // Colorless (light tint) below pH 8.2, pink above pH 9.8
      const baseColor = { r: 241, g: 245, b: 249, a: 0.12 }; // colorless water
      const targetColor = { r: 244, g: 63, b: 94, a: 0.70 }; // vibrant pink/rose
      const t = Math.max(0, Math.min(1, (ph - 8.2) / 1.6));
      const r = Math.round(baseColor.r + (targetColor.r - baseColor.r) * t);
      const g = Math.round(baseColor.g + (targetColor.g - baseColor.g) * t);
      const b = Math.round(baseColor.b + (targetColor.b - baseColor.b) * t);
      const a = baseColor.a + (targetColor.a - baseColor.a) * t;
      return `rgba(${r}, ${g}, ${b}, ${a})`;
    } else if (indicator === 'Methyl Orange') {
      // Red below pH 3.1, Yellow above pH 4.4, transition is Orange
      const redColor = { r: 239, g: 68, b: 68, a: 0.70 };
      const yellowColor = { r: 234, g: 179, b: 8, a: 0.70 };
      const t = Math.max(0, Math.min(1, (ph - 3.1) / 1.3));
      const r = Math.round(redColor.r + (yellowColor.r - redColor.r) * t);
      const g = Math.round(redColor.g + (yellowColor.g - redColor.g) * t);
      const b = Math.round(redColor.b + (yellowColor.b - redColor.b) * t);
      const a = redColor.a + (yellowColor.a - redColor.a) * t;
      return `rgba(${r}, ${g}, ${b}, ${a})`;
    } else if (indicator === 'Bromothymol Blue') {
      // Yellow below pH 6.0, Blue above pH 7.6, transition is Green
      const yellowColor = { r: 234, g: 179, b: 8, a: 0.70 };
      const blueColor = { r: 59, g: 130, b: 246, a: 0.70 };
      const t = Math.max(0, Math.min(1, (ph - 6.0) / 1.6));
      const r = Math.round(yellowColor.r + (blueColor.r - yellowColor.r) * t);
      const g = Math.round(yellowColor.g + (blueColor.g - yellowColor.g) * t);
      const b = Math.round(yellowColor.b + (blueColor.b - yellowColor.b) * t);
      const a = yellowColor.a + (blueColor.a - yellowColor.a) * t;
      return `rgba(${r}, ${g}, ${b}, ${a})`;
    }
    return 'rgba(241, 245, 249, 0.12)';
  }

  function startFlow() {
    isFlowing = true;
  }

  function stopFlow() {
    isFlowing = false;
  }

  function resetTitration() {
    isFlowing = false;
    titrationState.update(s => ({ ...s, addedVolume: 0.0 }));
    drops = [];
  }

  onMount(() => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const resize = () => {
      canvas.width = canvas.parentElement?.clientWidth || 600;
      canvas.height = canvas.parentElement?.clientHeight || 400;
    };
    window.addEventListener('resize', resize);
    resize();

    const loop = (timestamp: number) => {
      if (!lastTime) lastTime = timestamp;
      let dt = (timestamp - lastTime) / 1000;
      if (dt > 0.05) dt = 0.05;
      lastTime = timestamp;

      // Update added volume if flowing
      if (isFlowing) {
        titrationState.update(s => {
          const nextVol = Math.min(50.0, s.addedVolume + flowRate * dt);
          if (nextVol >= 50.0) {
            isFlowing = false;
          }
          return { ...s, addedVolume: nextVol };
        });

        // Spawn drops
        dropTimer += dt;
        if (dropTimer > 0.15) {
          drops.push({ y: 225, speed: 290 }); // Start right under burette tip
          dropTimer = 0;
        }
      }

      // Update drops physics
      const W = canvas.width;
      const H = canvas.height;
      const cx = W * 0.22; // Center of apparatus on canvas

      // Dynamic liquid level top in flask (rises as volume increases)
      const flaskBaseY = H - 65;
      const flaskTopY = flaskBaseY - 95;
      const totalVolume = $titrationState.analyteVolume + $titrationState.addedVolume; // 10 mL - 100 mL
      
      // Proportional liquid height (20px base + up to 55px added)
      const liquidHeight = 22 + (totalVolume / 100) * 52;
      const flaskLiquidTopY = flaskBaseY - liquidHeight;

      for (let i = drops.length - 1; i >= 0; i--) {
        drops[i].y += drops[i].speed * dt;
        // Check if drop hits the rising liquid surface in flask
        if (drops[i].y >= flaskLiquidTopY) {
          drops.splice(i, 1);
        }
      }

      // Clear canvas
      ctx.clearRect(0, 0, W, H);

      // --- Draw Left Side: Chemistry Apparatus ---

      // 1. Stand Base
      ctx.fillStyle = '#1e293b'; // slate-800
      ctx.beginPath();
      ctx.roundRect(cx - 60, H - 35, 120, 10, 3);
      ctx.fill();
      
      // Stand Rod
      ctx.fillStyle = '#475569'; // slate-600
      ctx.fillRect(cx - 3, 30, 6, H - 65);

      // Stand Metal Clamps
      ctx.fillStyle = '#64748b'; // slate-500
      ctx.fillRect(cx - 24, 75, 24, 6);
      ctx.fillRect(cx - 24, 175, 24, 6);

      // 2. Burette Assembly
      const buretteTopY = 35;
      const buretteBottomY = 210;
      const buretteW = 14;
      const liquidLevelY = buretteTopY + ($titrationState.addedVolume / 50.0) * (buretteBottomY - buretteTopY);

      // Burette Glass Tube (Background Glow & Backing)
      ctx.fillStyle = 'rgba(148, 163, 184, 0.08)';
      ctx.fillRect(cx - buretteW / 2, buretteTopY, buretteW, buretteBottomY - buretteTopY);

      // Liquid Column inside Burette
      if ($titrationState.addedVolume < 50.0) {
        const grad = ctx.createLinearGradient(cx - buretteW / 2, liquidLevelY, cx + buretteW / 2, liquidLevelY);
        grad.addColorStop(0, 'rgba(56, 189, 248, 0.45)'); // Light blue gradient
        grad.addColorStop(0.5, 'rgba(186, 230, 253, 0.35)');
        grad.addColorStop(1, 'rgba(56, 189, 248, 0.45)');
        
        ctx.fillStyle = grad;
        ctx.fillRect(cx - buretteW / 2, liquidLevelY, buretteW, buretteBottomY - liquidLevelY);
      }

      // Burette Outer Glass Borders
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.28)';
      ctx.lineWidth = 1.5;
      ctx.strokeRect(cx - buretteW / 2, buretteTopY, buretteW, buretteBottomY - buretteTopY);

      // Burette Calibration Markings
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.20)';
      ctx.lineWidth = 1.0;
      for (let ml = 0; ml <= 50; ml += 5) {
        const tickY = buretteTopY + (ml / 50.0) * (buretteBottomY - buretteTopY);
        const isMajor = ml % 10 === 0;
        const tickLen = isMajor ? 6 : 4;
        
        ctx.beginPath();
        ctx.moveTo(cx - buretteW / 2, tickY);
        ctx.lineTo(cx - buretteW / 2 + tickLen, tickY);
        ctx.stroke();

        if (isMajor) {
          ctx.fillStyle = 'rgba(148, 163, 184, 0.7)';
          ctx.font = '8px monospace';
          ctx.textAlign = 'left';
          ctx.textBaseline = 'middle';
          ctx.fillText(`${ml}`, cx + buretteW / 2 + 4, tickY);
        }
      }

      // Burette Tip Tube and Stopcock
      ctx.fillStyle = '#475569';
      ctx.fillRect(cx - 2.5, buretteBottomY, 5, 12); // Neck leading to stopcock
      
      // Stopcock body
      ctx.fillStyle = '#334155';
      ctx.fillRect(cx - 7, buretteBottomY + 12, 14, 8);

      // Stopcock handle (Green when closed, Red when open)
      ctx.fillStyle = isFlowing ? '#f43f5e' : '#10b981';
      ctx.save();
      ctx.translate(cx, buretteBottomY + 16);
      if (isFlowing) {
        ctx.rotate(Math.PI / 2); // Rotated open vertical
      }
      ctx.beginPath();
      ctx.roundRect(-8, -3, 16, 6, 2);
      ctx.fill();
      ctx.restore();

      // Lower tip tube
      ctx.fillStyle = '#475569';
      ctx.beginPath();
      ctx.moveTo(cx - 2, buretteBottomY + 20);
      ctx.lineTo(cx + 2, buretteBottomY + 20);
      ctx.lineTo(cx + 1, buretteBottomY + 28);
      ctx.lineTo(cx - 1, buretteBottomY + 28);
      ctx.closePath();
      ctx.fill();

      // 3. Draw falling liquid drops
      ctx.fillStyle = 'rgba(56, 189, 248, 0.65)';
      for (const drop of drops) {
        ctx.beginPath();
        ctx.arc(cx, drop.y, 2.5, 0, Math.PI * 2);
        ctx.fill();
        
        // Minor splash tail
        ctx.fillStyle = 'rgba(56, 189, 248, 0.35)';
        ctx.beginPath();
        ctx.moveTo(cx - 1.5, drop.y - 4);
        ctx.lineTo(cx + 1.5, drop.y - 4);
        ctx.lineTo(cx, drop.y);
        ctx.closePath();
        ctx.fill();
      }

      // 4. Erlenmeyer Flask
      const flaskW = 84;
      const flaskNeckW = 22;

      // Draw Flask Glass Shadow Background
      ctx.fillStyle = 'rgba(148, 163, 184, 0.03)';
      ctx.beginPath();
      ctx.moveTo(cx - flaskNeckW / 2, flaskTopY);
      ctx.lineTo(cx + flaskNeckW / 2, flaskTopY);
      ctx.lineTo(cx + flaskNeckW / 2, flaskTopY + 22);
      ctx.lineTo(cx + flaskW / 2, flaskBaseY);
      ctx.lineTo(cx - flaskW / 2, flaskBaseY);
      ctx.lineTo(cx - flaskNeckW / 2, flaskTopY + 22);
      ctx.closePath();
      ctx.fill();

      // Liquid in Erlenmeyer Flask (Reactively colored and rising)
      const activeColor = getIndicatorColor($titrationState.indicator, $currentPH);
      ctx.fillStyle = activeColor;
      
      const flaskLiqW = flaskNeckW + (flaskW - flaskNeckW) * ((flaskBaseY - flaskLiquidTopY) / (flaskBaseY - flaskTopY));
      ctx.beginPath();
      ctx.moveTo(cx - flaskLiqW / 2, flaskLiquidTopY);
      ctx.lineTo(cx + flaskLiqW / 2, flaskLiquidTopY);
      ctx.lineTo(cx + flaskW / 2 - 2, flaskBaseY - 2);
      ctx.lineTo(cx - flaskW / 2 + 2, flaskBaseY - 2);
      ctx.closePath();
      ctx.fill();

      // Draw Flask Glass Outline (over liquid)
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.32)';
      ctx.lineWidth = 1.8;
      ctx.beginPath();
      ctx.moveTo(cx - flaskNeckW / 2, flaskTopY);
      ctx.lineTo(cx + flaskNeckW / 2, flaskTopY);
      ctx.lineTo(cx + flaskNeckW / 2, flaskTopY + 22);
      ctx.lineTo(cx + flaskW / 2, flaskBaseY);
      ctx.lineTo(cx - flaskW / 2, flaskBaseY);
      ctx.lineTo(cx - flaskNeckW / 2, flaskTopY + 22);
      ctx.closePath();
      ctx.stroke();

      // Flask Lip (rim)
      ctx.beginPath();
      ctx.ellipse(cx, flaskTopY, flaskNeckW / 2 + 2, 2, 0, 0, Math.PI * 2);
      ctx.stroke();

      // Liquid Surface Curve / Meniscus in Flask
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
      ctx.lineWidth = 1.0;
      ctx.beginPath();
      ctx.ellipse(cx, flaskLiquidTopY, flaskLiqW / 2, 1.5, 0, 0, Math.PI * 2);
      ctx.stroke();

      // --- Draw Right Side: Titration Curve Graph ---
      const graphX = W * 0.44;
      const graphY = H * 0.15;
      const graphW = W * 0.51;
      const graphH = H * 0.67;

      // Dark Neon Graph Frame
      ctx.fillStyle = '#020617'; // slate-950
      ctx.fillRect(graphX, graphY, graphW, graphH);
      ctx.strokeStyle = '#1e293b'; // slate-800
      ctx.lineWidth = 1.5;
      ctx.strokeRect(graphX, graphY, graphW, graphH);

      // Graph Grid Lines (pH & Volume)
      ctx.strokeStyle = 'rgba(30, 41, 59, 0.4)';
      ctx.lineWidth = 1.0;
      // Volume Grid (x-axis) - 0 to 50 mL
      for (let v = 10; v < 50; v += 10) {
        const gx = graphX + (v / 50) * graphW;
        ctx.beginPath();
        ctx.moveTo(gx, graphY);
        ctx.lineTo(gx, graphY + graphH);
        ctx.stroke();
      }
      // pH Grid (y-axis) - 0 to 14
      for (let p = 2; p <= 12; p += 2) {
        const gy = graphY + graphH - (p / 14) * graphH;
        ctx.beginPath();
        ctx.moveTo(graphX, gy);
        ctx.lineTo(graphX + graphW, gy);
        ctx.stroke();
      }

      // Graph Axes Ticks and Labels
      ctx.fillStyle = '#64748b'; // slate-500
      ctx.font = '9px monospace';
      // pH Y-axis labels
      ctx.textAlign = 'right';
      ctx.textBaseline = 'middle';
      for (let p = 0; p <= 14; p += 2) {
        const gy = graphY + graphH - (p / 14) * graphH;
        ctx.fillText(`${p}`, graphX - 8, gy);
      }
      // Volume X-axis labels
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';
      for (let v = 0; v <= 50; v += 10) {
        const gx = graphX + (v / 50) * graphW;
        ctx.fillText(`${v}`, gx, graphY + graphH + 8);
      }

      // Axis Titles
      ctx.fillStyle = '#94a3b8'; // slate-400
      ctx.font = 'bold 10px sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText('Volume Titrant Added (mL)', graphX + graphW / 2, graphY + graphH + 24);

      // Rotated Y-axis title
      ctx.save();
      ctx.translate(graphX - 25, graphY + graphH / 2);
      ctx.rotate(-Math.PI / 2);
      ctx.fillText('pH Value', 0, 0);
      ctx.restore();

      // Draw the full analytical titration curve line
      const M1 = $titrationState.titrantConc;
      const V2 = $titrationState.analyteVolume;
      const M2 = $titrationState.analyteConc;

      ctx.strokeStyle = '#06b6d4'; // Cyan neon curve
      ctx.lineWidth = 2.0;
      ctx.beginPath();
      for (let v = 0; v <= 50.0; v += 0.25) {
        const phVal = calculatePH(v, M1, V2, M2);
        const gx = graphX + (v / 50.0) * graphW;
        const gy = graphY + graphH - (phVal / 14.0) * graphH;
        if (v === 0) {
          ctx.moveTo(gx, gy);
        } else {
          ctx.lineTo(gx, gy);
        }
      }
      ctx.stroke();

      // Draw active volume/pH point marker
      const curX = graphX + ($titrationState.addedVolume / 50.0) * graphW;
      const curY = graphY + graphH - ($currentPH / 14.0) * graphH;

      // Dashed crosshairs to axes
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
      ctx.lineWidth = 1.0;
      ctx.setLineDash([3, 3]);
      
      // Vertical crosshair
      ctx.beginPath();
      ctx.moveTo(curX, curY);
      ctx.lineTo(curX, graphY + graphH);
      ctx.stroke();
      
      // Horizontal crosshair
      ctx.beginPath();
      ctx.moveTo(curX, curY);
      ctx.lineTo(graphX, curY);
      ctx.stroke();
      ctx.setLineDash([]);

      // Glow effect for active pointer
      ctx.save();
      ctx.shadowColor = '#06b6d4';
      ctx.shadowBlur = 10;
      ctx.fillStyle = '#22d3ee';
      ctx.beginPath();
      ctx.arc(curX, curY, 6, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();

      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.arc(curX, curY, 6, 0, Math.PI * 2);
      ctx.stroke();

      // Equivalence Point line overlay (Theoretical Equivalence point is when V_added * M_titrant = V_analyte * M_analyte)
      const vEq = V2 * (M2 / M1);
      if (vEq <= 50.0) {
        const eqX = graphX + (vEq / 50.0) * graphW;
        ctx.strokeStyle = 'rgba(239, 68, 68, 0.35)'; // Faint red dashed line
        ctx.lineWidth = 1.0;
        ctx.setLineDash([2, 4]);
        ctx.beginPath();
        ctx.moveTo(eqX, graphY);
        ctx.lineTo(eqX, graphY + graphH);
        ctx.stroke();
        ctx.setLineDash([]);

        // Label for Equivalence Point
        ctx.fillStyle = 'rgba(239, 68, 68, 0.6)';
        ctx.font = '8px sans-serif';
        ctx.textAlign = 'left';
        ctx.fillText(`Eq Point (${vEq.toFixed(1)} mL)`, eqX + 4, graphY + 12);
      }

      // Live Stats Status Overlay Box inside the graph
      ctx.fillStyle = 'rgba(15, 23, 42, 0.8)'; // slate-900 back
      ctx.beginPath();
      ctx.roundRect(graphX + 12, graphY + 12, 140, 52, 6);
      ctx.fill();
      ctx.strokeStyle = '#334155'; // slate-700
      ctx.lineWidth = 1.0;
      ctx.strokeRect(graphX + 12, graphY + 12, 140, 52);

      ctx.fillStyle = '#e2e8f0'; // slate-200
      ctx.font = '9px monospace';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'top';
      ctx.fillText(`VOLUME : ${$titrationState.addedVolume.toFixed(2)} mL`, graphX + 20, graphY + 18);
      ctx.fillText(`PH VAL : ${$currentPH.toFixed(2)}`, graphX + 20, graphY + 30);
      ctx.fillText(`EQ VOL : ${vEq.toFixed(2)} mL`, graphX + 20, graphY + 42);

      animationFrameId = requestAnimationFrame(loop);
    };

    animationFrameId = requestAnimationFrame(loop);

    return () => {
      window.removeEventListener('resize', resize);
      cancelAnimationFrame(animationFrameId);
    };
  });
</script>

<div class="relative w-full h-full">
  <canvas bind:this={canvas} class="w-full h-full block bg-slate-950"></canvas>
  
  <!-- Overlay Controls -->
  <div class="absolute bottom-4 left-4 flex gap-2 z-10">
    {#if !isFlowing}
      <button 
        class="bg-emerald-600 hover:bg-emerald-700 text-white font-semibold py-1.5 px-3 rounded text-xs transition-all flex items-center gap-1 shadow-md cursor-pointer border border-emerald-500"
        on:click={startFlow}
        disabled={$titrationState.addedVolume >= 50.0}
      >
        ▶ START FLOW
      </button>
    {:else}
      <button 
        class="bg-amber-600 hover:bg-amber-700 text-white font-semibold py-1.5 px-3 rounded text-xs transition-all flex items-center gap-1 shadow-md cursor-pointer border border-amber-500"
        on:click={stopFlow}
      >
        ⏸ PAUSE FLOW
      </button>
    {/if}
    <button 
      class="bg-slate-700 hover:bg-slate-600 text-white font-semibold py-1.5 px-3 rounded text-xs transition-all flex items-center gap-1 shadow-md cursor-pointer border border-slate-600"
      on:click={resetTitration}
    >
      ⟲ RESET
    </button>
  </div>
</div>
