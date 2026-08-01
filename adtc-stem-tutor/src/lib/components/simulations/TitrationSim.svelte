<script lang="ts">
  import { onMount } from 'svelte';
  import { titrationState, currentPH } from '$lib/stores/labStore';

  let apparatusCanvas: HTMLCanvasElement;
  let graphCanvas: HTMLCanvasElement;
  let animationFrameId: number;
  let lastTime = 0;
  
  // Animation and Flow state
  export let isFlowing = false;
  export let flowRate = 1.0; // mL per second
  let drops: Array<{ y: number; speed: number; volume?: number }> = [];
  let dropTimer = 0;
  
  // High-fidelity animation visual additions
  let ripples: Array<{ y: number; r: number; maxR: number; alpha: number }> = [];
  let swirls: Array<{ x: number; y: number; vx: number; vy: number; radius: number; maxRadius: number; alpha: number; color: string }> = [];
  let stirrerAngle = 0;
  let stirrerSpeed = 1.5; // multiplier when active
  
  // Layout measurements (updated dynamically inside the loop)
  let cx = 0;
  let flaskBaseY = 0;
  let flaskTopY = 0;
  let tipY = 0;
  let stopcockY = 0;
  let buretteBottomY = 0;
  let buretteTopY = 0;
  
  // Graph tooltip interactivity
  let graphMouseX: number | null = null;
  let graphMouseY: number | null = null;

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
      const baseColor = { r: 226, g: 232, b: 240, a: 0.05 }; // colorless water
      const targetColor = { r: 244, g: 63, b: 94, a: 0.72 }; // vibrant pink/rose
      const t = Math.max(0, Math.min(1, (ph - 8.2) / 1.6));
      const r = Math.round(baseColor.r + (targetColor.r - baseColor.r) * t);
      const g = Math.round(baseColor.g + (targetColor.g - baseColor.g) * t);
      const b = Math.round(baseColor.b + (targetColor.b - baseColor.b) * t);
      const a = baseColor.a + (targetColor.a - baseColor.a) * t;
      return `rgba(${r}, ${g}, ${b}, ${a})`;
    } else if (indicator === 'Methyl Orange') {
      // Red below pH 3.1, Yellow above pH 4.4, transition is Orange
      const redColor = { r: 239, g: 68, b: 68, a: 0.72 };
      const yellowColor = { r: 234, g: 179, b: 8, a: 0.72 };
      const t = Math.max(0, Math.min(1, (ph - 3.1) / 1.3));
      const r = Math.round(redColor.r + (yellowColor.r - redColor.r) * t);
      const g = Math.round(redColor.g + (yellowColor.g - redColor.g) * t);
      const b = Math.round(redColor.b + (yellowColor.b - redColor.b) * t);
      const a = redColor.a + (yellowColor.a - redColor.a) * t;
      return `rgba(${r}, ${g}, ${b}, ${a})`;
    } else if (indicator === 'Bromothymol Blue') {
      // Yellow below pH 6.0, Green around 6.8, Blue above pH 7.6
      if (ph < 6.0) {
        return 'rgba(234, 179, 8, 0.72)';
      } else if (ph > 7.6) {
        return 'rgba(59, 130, 246, 0.72)';
      } else if (ph <= 6.8) {
        const t = (ph - 6.0) / 0.8;
        const r = Math.round(234 + (34 - 234) * t);
        const g = Math.round(179 + (197 - 179) * t);
        const b = Math.round(8 + (94 - 8) * t);
        return `rgba(${r}, ${g}, ${b}, 0.72)`;
      } else {
        const t = (ph - 6.8) / 0.8;
        const r = Math.round(34 + (59 - 34) * t);
        const g = Math.round(197 + (130 - 197) * t);
        const b = Math.round(94 + (246 - 94) * t);
        return `rgba(${r}, ${g}, ${b}, 0.72)`;
      }
    }
    return 'rgba(226, 232, 240, 0.05)';
  }

  function getSplashSwirlColor(indicator: string) {
    if (indicator === 'Phenolphthalein') {
      return 'rgba(244, 63, 94, alpha)';
    } else if (indicator === 'Methyl Orange') {
      return 'rgba(234, 179, 8, alpha)'; // Turns yellow (basic)
    } else if (indicator === 'Bromothymol Blue') {
      return 'rgba(59, 130, 246, alpha)'; // Turns blue (basic)
    }
    return 'rgba(255, 255, 255, alpha)';
  }

  export function startFlow() {
    if ($titrationState.addedVolume >= 50.0) return;
    isFlowing = true;
  }

  export function stopFlow() {
    isFlowing = false;
  }

  export function resetTitration() {
    isFlowing = false;
    titrationState.update(s => ({ ...s, addedVolume: 0.0 }));
    drops = [];
    ripples = [];
    swirls = [];
  }

  export function triggerManualAddition(amount: number) {
    if ($titrationState.addedVolume >= 50.0) return;
    
    // We add drops staggered upwards from the tip
    if (amount === 0.05) {
      drops.push({ y: tipY, speed: 320, volume: 0.05 });
    } else if (amount === 0.20) {
      for (let i = 0; i < 4; i++) {
        drops.push({ y: tipY - i * 18, speed: 320, volume: 0.05 });
      }
    } else if (amount === 1.00) {
      for (let i = 0; i < 5; i++) {
        drops.push({ y: tipY - i * 15, speed: 320, volume: 0.20 });
      }
    } else if (amount === 5.00) {
      for (let i = 0; i < 10; i++) {
        drops.push({ y: tipY - i * 12, speed: 320, volume: 0.50 });
      }
    }
  }

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

  function handleGraphMouseMove(e: MouseEvent) {
    if (!graphCanvas) return;
    const rect = graphCanvas.getBoundingClientRect();
    graphMouseX = e.clientX - rect.left;
    graphMouseY = e.clientY - rect.top;
  }
  
  function handleGraphMouseLeave() {
    graphMouseX = null;
    graphMouseY = null;
  }

  function handleApparatusMouseMove(e: MouseEvent) {
    if (!apparatusCanvas) return;
    const rect = apparatusCanvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    // Check click distance to stopcock (at cx, stopcockY)
    const dist = Math.hypot(x - cx, y - stopcockY);
    if (dist < 15) {
      apparatusCanvas.style.cursor = 'pointer';
    } else {
      apparatusCanvas.style.cursor = 'default';
    }
  }

  function handleApparatusClick(e: MouseEvent) {
    if (!apparatusCanvas) return;
    const rect = apparatusCanvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    const dist = Math.hypot(x - cx, y - stopcockY);
    if (dist < 15) {
      if (isFlowing) {
        stopFlow();
      } else {
        startFlow();
      }
    }
  }

  function drawZoomView(ctx: CanvasRenderingContext2D, W: number, H: number) {
    const zoomX = W - 55;
    const zoomY = 70;
    const zoomRadius = 40;
    
    if (W < 180) return; // Hide if canvas is too narrow
    
    ctx.save();
    
    // Circular mask
    ctx.beginPath();
    ctx.arc(zoomX, zoomY, zoomRadius, 0, Math.PI * 2);
    ctx.clip();
    
    // Background
    ctx.fillStyle = '#020617';
    ctx.beginPath();
    ctx.arc(zoomX, zoomY, zoomRadius, 0, Math.PI * 2);
    ctx.fill();
    
    // Zoomed burette tube
    const zoomScale = 32; // px per mL
    const v = $titrationState.addedVolume;
    
    ctx.fillStyle = 'rgba(148, 163, 184, 0.05)';
    ctx.fillRect(zoomX - 16, zoomY - zoomRadius, 32, zoomRadius * 2);
    
    // Grid scale
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.22)';
    ctx.lineWidth = 0.8;
    
    const startVal = Math.max(0, Math.floor(v - 1.5));
    const endVal = Math.min(50, Math.ceil(v + 1.5));
    
    for (let tick = startVal * 10; tick <= endVal * 10; tick++) {
      const volMark = tick / 10;
      const markY = zoomY + (volMark - v) * zoomScale;
      
      if (markY < zoomY - zoomRadius || markY > zoomY + zoomRadius) continue;
      
      const isMajor = tick % 10 === 0;
      const isHalf = tick % 5 === 0;
      
      const tickLen = isMajor ? 9 : (isHalf ? 6 : 3);
      ctx.beginPath();
      ctx.moveTo(zoomX - 16, markY);
      ctx.lineTo(zoomX - 16 + tickLen, markY);
      ctx.stroke();
      
      if (isMajor) {
        ctx.fillStyle = 'rgba(148, 163, 184, 0.8)';
        ctx.font = 'bold 7px monospace';
        ctx.textAlign = 'left';
        ctx.textBaseline = 'middle';
        ctx.fillText(`${volMark.toFixed(0)}`, zoomX + 2, markY);
      }
    }
    
    // Liquid level (locked in center)
    if (v < 50) {
      ctx.fillStyle = 'rgba(56, 189, 248, 0.22)';
      ctx.beginPath();
      ctx.moveTo(zoomX - 16, zoomY);
      ctx.bezierCurveTo(zoomX - 8, zoomY + 3, zoomX + 8, zoomY + 3, zoomX + 16, zoomY);
      ctx.lineTo(zoomX + 16, zoomY + zoomRadius);
      ctx.lineTo(zoomX - 16, zoomY + zoomRadius);
      ctx.closePath();
      ctx.fill();
      
      ctx.strokeStyle = 'rgba(56, 189, 248, 0.65)';
      ctx.lineWidth = 1.2;
      ctx.beginPath();
      ctx.moveTo(zoomX - 16, zoomY);
      ctx.bezierCurveTo(zoomX - 8, zoomY + 3, zoomX + 8, zoomY + 3, zoomX + 16, zoomY);
      ctx.stroke();
    }
    
    // Glass tube side borders in zoom
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.35)';
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.moveTo(zoomX - 16, zoomY - zoomRadius);
    ctx.lineTo(zoomX - 16, zoomY + zoomRadius);
    ctx.moveTo(zoomX + 16, zoomY - zoomRadius);
    ctx.lineTo(zoomX + 16, zoomY + zoomRadius);
    ctx.stroke();
    
    ctx.restore();
    
    // Magnifying glass border
    ctx.strokeStyle = '#475569'; // steel border
    ctx.lineWidth = 2.0;
    ctx.beginPath();
    ctx.arc(zoomX, zoomY, zoomRadius, 0, Math.PI * 2);
    ctx.stroke();
    
    ctx.fillStyle = '#64748b';
    ctx.font = '8px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('MENISCUS', zoomX, zoomY + zoomRadius + 11);
  }

  function drawIndicatorBand(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number) {
    let trMin = 0;
    let trMax = 0;
    let trColorStart = '';
    let trColorEnd = '';
    
    if ($titrationState.indicator === 'Phenolphthalein') {
      trMin = 8.2;
      trMax = 9.8;
      trColorStart = 'rgba(244, 63, 94, 0.0)';
      trColorEnd = 'rgba(244, 63, 94, 0.15)';
    } else if ($titrationState.indicator === 'Methyl Orange') {
      trMin = 3.1;
      trMax = 4.4;
      trColorStart = 'rgba(239, 68, 68, 0.12)';
      trColorEnd = 'rgba(234, 179, 8, 0.12)';
    } else if ($titrationState.indicator === 'Bromothymol Blue') {
      trMin = 6.0;
      trMax = 7.6;
      trColorStart = 'rgba(234, 179, 8, 0.12)';
      trColorEnd = 'rgba(59, 130, 246, 0.12)';
    }
    
    if (trMin > 0) {
      const yStart = y + h - (trMax / 14.0) * h;
      const yEnd = y + h - (trMin / 14.0) * h;
      
      const bandGrad = ctx.createLinearGradient(x, yStart, x, yEnd);
      bandGrad.addColorStop(0, trColorStart);
      if ($titrationState.indicator === 'Bromothymol Blue') {
        bandGrad.addColorStop(0.5, 'rgba(34, 197, 94, 0.12)'); // green transition
      }
      bandGrad.addColorStop(1, trColorEnd);
      
      ctx.fillStyle = bandGrad;
      ctx.fillRect(x, yStart, w, yEnd - yStart);
      
      // Dashed boundary lines
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
      ctx.lineWidth = 0.8;
      ctx.setLineDash([3, 3]);
      ctx.beginPath();
      ctx.moveTo(x, yStart);
      ctx.lineTo(x + w, yStart);
      ctx.moveTo(x, yEnd);
      ctx.lineTo(x + w, yEnd);
      ctx.stroke();
      ctx.setLineDash([]);
      
      // Text labels
      ctx.fillStyle = 'rgba(255, 255, 255, 0.35)';
      ctx.font = 'bold 7px sans-serif';
      ctx.textAlign = 'right';
      ctx.fillText(`pH ${trMax}`, x + w - 4, yStart - 2);
      ctx.fillText(`pH ${trMin}`, x + w - 4, yEnd + 7);
    }
  }

  function drawHoverTooltip(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, M1: number, V2: number, M2: number) {
    if (graphMouseX === null || graphMouseX < x || graphMouseX > x + w) return;
    
    const hoverV = ((graphMouseX - x) / w) * 50.0;
    const hoverPH = calculatePH(hoverV, M1, V2, M2);
    
    const hgx = x + (hoverV / 50.0) * w;
    const hgy = y + h - (hoverPH / 14.0) * h;
    
    // Draw vertical indicator line
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.25)';
    ctx.lineWidth = 0.8;
    ctx.beginPath();
    ctx.moveTo(hgx, y);
    ctx.lineTo(hgx, y + h);
    ctx.stroke();
    
    // Circle indicator on the curve
    ctx.fillStyle = '#ffffff';
    ctx.beginPath();
    ctx.arc(hgx, hgy, 3, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = '#06b6d4';
    ctx.lineWidth = 1;
    ctx.stroke();
    
    // Tooltip box positioning
    const padX = hgx > x + w / 2 ? -75 : 12;
    const boxY = Math.max(y + 5, Math.min(y + h - 45, hgy - 15));
    
    ctx.fillStyle = 'rgba(15, 23, 42, 0.9)'; // Slate-900 back
    ctx.strokeStyle = 'rgba(56, 189, 248, 0.8)'; // Cyan border
    ctx.lineWidth = 1.0;
    ctx.beginPath();
    ctx.roundRect(hgx + padX, boxY, 63, 30, 4);
    ctx.fill();
    ctx.stroke();
    
    ctx.fillStyle = '#f8fafc';
    ctx.font = 'bold 7px monospace';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    ctx.fillText(`V : ${hoverV.toFixed(2)}`, hgx + padX + 5, boxY + 5);
    ctx.fillText(`pH: ${hoverPH.toFixed(2)}`, hgx + padX + 5, boxY + 16);
  }

  onMount(() => {
    const resize = () => {
      const dpr = window.devicePixelRatio || 1;
      
      const appParent = apparatusCanvas?.parentElement;
      if (appParent) {
        const w = appParent.clientWidth || 300;
        const h = appParent.clientHeight || 450;
        apparatusCanvas.width = w * dpr;
        apparatusCanvas.height = h * dpr;
        apparatusCanvas.style.width = `${w}px`;
        apparatusCanvas.style.height = `${h}px`;
      }

      const graphParent = graphCanvas?.parentElement;
      if (graphParent) {
        const w = graphParent.clientWidth || 400;
        const h = graphParent.clientHeight || 250;
        graphCanvas.width = w * dpr;
        graphCanvas.height = h * dpr;
        graphCanvas.style.width = `${w}px`;
        graphCanvas.style.height = `${h}px`;
      }
    };
    
    window.addEventListener('resize', resize);
    resize();

    const loop = (timestamp: number) => {
      if (!lastTime) lastTime = timestamp;
      let dt = (timestamp - lastTime) / 1000;
      if (dt > 0.05) dt = 0.05;
      lastTime = timestamp;

      const dpr = window.devicePixelRatio || 1;
      
      // Update dimensions
      const appW = apparatusCanvas.width / dpr;
      const appH = apparatusCanvas.height / dpr;
      cx = appW / 2;
      flaskBaseY = appH - 60;
      flaskTopY = flaskBaseY - 95;
      tipY = flaskTopY - 35;
      stopcockY = tipY - 14;
      buretteBottomY = stopcockY - 14;
      buretteTopY = 30;

      // Update titration state from continuous flow
      if (isFlowing) {
        if (flowRate < 2.0) {
          // Dripping mode
          dropTimer += dt;
          const spawnInterval = 0.05 / flowRate;
          while (dropTimer >= spawnInterval && $titrationState.addedVolume < 50.0) {
            drops.push({ y: tipY, speed: 320, volume: 0.05 });
            dropTimer -= spawnInterval;
          }
        } else {
          // Streaming mode (continuous stream)
          titrationState.update(s => {
            const nextVol = Math.min(50.0, s.addedVolume + flowRate * dt);
            if (nextVol >= 50.0) isFlowing = false;
            return { ...s, addedVolume: nextVol };
          });
          
          // Spawn ripples and splash swirls continuously
          if (Math.random() < 0.3) {
            const totalVol = $titrationState.analyteVolume + $titrationState.addedVolume;
            const liquidHeight = 22 + (totalVol / 100) * 55;
            const flaskLiquidTopY = flaskBaseY - liquidHeight;
            
            ripples.push({
              y: flaskLiquidTopY,
              r: 2,
              maxR: 25,
              alpha: 0.6
            });
            
            const splashColor = getSplashSwirlColor($titrationState.indicator);
            swirls.push({
              x: cx + (Math.random() - 0.5) * 8,
              y: flaskLiquidTopY + 3,
              vx: (Math.random() - 0.5) * 12,
              vy: Math.random() * 20 + 15,
              radius: 2,
              maxRadius: 12,
              alpha: 0.7,
              color: splashColor
            });
          }
        }
      }

      // Update stirrer animation
      if (stirrerSpeed > 0) {
        stirrerAngle += (stirrerSpeed * 10 * dt);
      }

      // Update drops physics
      const totalVolume = $titrationState.analyteVolume + $titrationState.addedVolume;
      const liquidHeight = 22 + (totalVolume / 100) * 55;
      const flaskLiquidTopY = flaskBaseY - liquidHeight;

      for (let i = drops.length - 1; i >= 0; i--) {
        const drop = drops[i];
        drop.y += drop.speed * dt;
        
        // If drop hits liquid
        if (drop.y >= flaskLiquidTopY) {
          drops.splice(i, 1);
          
          if (drop.volume !== undefined) {
            const vol = drop.volume;
            titrationState.update(s => {
              const nextVol = Math.min(50.0, s.addedVolume + vol);
              if (nextVol >= 50.0) isFlowing = false;
              return { ...s, addedVolume: nextVol };
            });
          }
          
          // Trigger splash ripple
          const flaskLiqW = 26 + (90 - 26) * ((flaskBaseY - flaskLiquidTopY) / (flaskBaseY - flaskTopY));
          ripples.push({
            y: flaskLiquidTopY,
            r: 1,
            maxR: flaskLiqW * 0.45,
            alpha: 0.85
          });
          
          // Trigger swirl
          const splashColor = getSplashSwirlColor($titrationState.indicator);
          swirls.push({
            x: cx + (Math.random() - 0.5) * 6,
            y: flaskLiquidTopY + 2,
            vx: (Math.random() - 0.5) * 10,
            vy: Math.random() * 15 + 10,
            radius: 2.5,
            maxRadius: 14,
            alpha: 0.8,
            color: splashColor
          });
        }
      }

      // Render Apparatus Canvas
      const appCtx = apparatusCanvas?.getContext('2d');
      if (appCtx) {
        appCtx.setTransform(1, 0, 0, 1, 0, 0);
        appCtx.clearRect(0, 0, apparatusCanvas.width, apparatusCanvas.height);
        appCtx.scale(dpr, dpr);

        // 1. Draw Stand Base
        const baseGrad = appCtx.createLinearGradient(cx - 70, flaskBaseY + 25, cx + 70, flaskBaseY + 25);
        baseGrad.addColorStop(0, '#1e293b');
        baseGrad.addColorStop(0.5, '#334155');
        baseGrad.addColorStop(1, '#1e293b');
        appCtx.fillStyle = baseGrad;
        appCtx.beginPath();
        appCtx.roundRect(cx - 70, flaskBaseY + 22, 140, 10, 3);
        appCtx.fill();
        appCtx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
        appCtx.lineWidth = 1;
        appCtx.stroke();

        // Stand Rod
        const rodGrad = appCtx.createLinearGradient(cx - 38, 20, cx - 32, 20);
        rodGrad.addColorStop(0, '#334155');
        rodGrad.addColorStop(0.5, '#64748b');
        rodGrad.addColorStop(1, '#334155');
        appCtx.fillStyle = rodGrad;
        appCtx.fillRect(cx - 36, 20, 6, flaskBaseY + 22);

        // Clamps holding the burette
        appCtx.fillStyle = '#475569';
        appCtx.fillRect(cx - 30, 75, 30, 5);
        appCtx.fillRect(cx - 30, 180, 30, 5);
        // Claw shapes
        appCtx.strokeStyle = '#64748b';
        appCtx.lineWidth = 2.0;
        appCtx.beginPath();
        appCtx.arc(cx, 77.5, 9, 0, Math.PI * 2);
        appCtx.stroke();
        appCtx.beginPath();
        appCtx.arc(cx, 182.5, 9, 0, Math.PI * 2);
        appCtx.stroke();

        // 2. Draw Burette tube
        const buretteW = 14;
        const liquidLevelY = buretteTopY + ($titrationState.addedVolume / 50.0) * (buretteBottomY - buretteTopY);

        // Glass background
        appCtx.fillStyle = 'rgba(148, 163, 184, 0.05)';
        appCtx.fillRect(cx - buretteW / 2, buretteTopY, buretteW, buretteBottomY - buretteTopY);

        // Liquid inside burette
        if ($titrationState.addedVolume < 50.0) {
          const liquidGrad = appCtx.createLinearGradient(cx - buretteW / 2, liquidLevelY, cx + buretteW / 2, liquidLevelY);
          liquidGrad.addColorStop(0, 'rgba(56, 189, 248, 0.28)');
          liquidGrad.addColorStop(0.5, 'rgba(186, 230, 253, 0.18)');
          liquidGrad.addColorStop(1, 'rgba(56, 189, 248, 0.28)');
          appCtx.fillStyle = liquidGrad;
          appCtx.fillRect(cx - buretteW / 2, liquidLevelY, buretteW, buretteBottomY - liquidLevelY);
          
          // Meniscus curve
          appCtx.strokeStyle = 'rgba(56, 189, 248, 0.5)';
          appCtx.lineWidth = 1.0;
          appCtx.beginPath();
          appCtx.ellipse(cx, liquidLevelY, buretteW / 2, 1.5, 0, 0, Math.PI, false);
          appCtx.stroke();
        }

        // Glass tube borders
        appCtx.strokeStyle = 'rgba(255, 255, 255, 0.25)';
        appCtx.lineWidth = 1.2;
        appCtx.beginPath();
        appCtx.moveTo(cx - buretteW / 2, buretteTopY);
        appCtx.lineTo(cx - buretteW / 2, buretteBottomY);
        appCtx.moveTo(cx + buretteW / 2, buretteTopY);
        appCtx.lineTo(cx + buretteW / 2, buretteBottomY);
        appCtx.stroke();

        // Calibration marks
        appCtx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
        appCtx.lineWidth = 0.8;
        for (let ml = 0; ml <= 50; ml += 5) {
          const tickY = buretteTopY + (ml / 50.0) * (buretteBottomY - buretteTopY);
          const isMajor = ml % 10 === 0;
          const tickLen = isMajor ? 6 : 4;
          
          appCtx.beginPath();
          appCtx.moveTo(cx - buretteW / 2, tickY);
          appCtx.lineTo(cx - buretteW / 2 + tickLen, tickY);
          appCtx.stroke();

          if (isMajor) {
            appCtx.fillStyle = 'rgba(148, 163, 184, 0.7)';
            appCtx.font = '7px monospace';
            appCtx.textAlign = 'left';
            appCtx.textBaseline = 'middle';
            appCtx.fillText(`${ml}`, cx + buretteW / 2 + 3, tickY);
          }
        }

        // Stopcock neck, body and tip
        appCtx.fillStyle = '#475569';
        appCtx.fillRect(cx - 2, buretteBottomY, 4, 14); // Neck
        
        // Stopcock base body
        appCtx.fillStyle = '#1e293b';
        appCtx.fillRect(cx - 6, stopcockY - 4, 12, 8);
        appCtx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
        appCtx.strokeRect(cx - 6, stopcockY - 4, 12, 8);

        // Stopcock handle (rotates based on isFlowing)
        appCtx.save();
        appCtx.translate(cx, stopcockY);
        if (isFlowing) {
          appCtx.rotate(Math.PI / 2);
        }
        appCtx.fillStyle = isFlowing ? '#3b82f6' : '#64748b'; // blue when flowing, gray when closed
        appCtx.beginPath();
        appCtx.roundRect(-8, -3, 16, 6, 2);
        appCtx.fill();
        appCtx.fillStyle = '#ffffff';
        appCtx.beginPath();
        appCtx.arc(0, 0, 1.5, 0, Math.PI * 2);
        appCtx.fill();
        appCtx.restore();

        // Tip glass narrowing down
        appCtx.fillStyle = '#475569';
        appCtx.beginPath();
        appCtx.moveTo(cx - 2, stopcockY + 4);
        appCtx.lineTo(cx + 2, stopcockY + 4);
        appCtx.lineTo(cx + 1, tipY);
        appCtx.lineTo(cx - 1, tipY);
        appCtx.closePath();
        appCtx.fill();

        // 3. Draw drops and streaming liquid
        if (isFlowing && flowRate >= 2.0 && $titrationState.addedVolume < 50.0) {
          // Draw stream
          const streamGrad = appCtx.createLinearGradient(cx - 1, tipY, cx + 1, tipY);
          streamGrad.addColorStop(0, 'rgba(56, 189, 248, 0.45)');
          streamGrad.addColorStop(1, 'rgba(56, 189, 248, 0.15)');
          appCtx.fillStyle = streamGrad;
          appCtx.fillRect(cx - 1, tipY, 2, flaskLiquidTopY - tipY);
        } else {
          // Draw drops
          appCtx.fillStyle = 'rgba(56, 189, 248, 0.6)';
          for (const d of drops) {
            if (d.y < tipY) continue; // Not spawned yet (staggered delay)
            appCtx.beginPath();
            appCtx.arc(cx, d.y, 2.0, 0, Math.PI * 2);
            appCtx.fill();
            
            // Drop tail
            appCtx.fillStyle = 'rgba(56, 189, 248, 0.3)';
            appCtx.beginPath();
            appCtx.moveTo(cx - 1.2, d.y - 3);
            appCtx.lineTo(cx + 1.2, d.y - 3);
            appCtx.lineTo(cx, d.y);
            appCtx.closePath();
            appCtx.fill();
          }
        }

        // 4. Erlenmeyer Flask
        const flaskW = 90;
        const flaskNeckW = 26;

        // Shadow Background
        appCtx.fillStyle = 'rgba(148, 163, 184, 0.02)';
        appCtx.beginPath();
        appCtx.moveTo(cx - flaskNeckW / 2, flaskTopY);
        appCtx.lineTo(cx + flaskNeckW / 2, flaskTopY);
        appCtx.lineTo(cx + flaskNeckW / 2, flaskTopY + 20);
        appCtx.lineTo(cx + flaskW / 2, flaskBaseY);
        appCtx.lineTo(cx - flaskW / 2, flaskBaseY);
        appCtx.lineTo(cx - flaskNeckW / 2, flaskTopY + 20);
        appCtx.closePath();
        appCtx.fill();

        // Liquid in Flask
        const activeColor = getIndicatorColor($titrationState.indicator, $currentPH);
        appCtx.fillStyle = activeColor;
        
        const flaskLiqW = flaskNeckW + (flaskW - flaskNeckW) * ((flaskBaseY - flaskLiquidTopY) / (flaskBaseY - flaskTopY));
        appCtx.beginPath();
        appCtx.moveTo(cx - flaskLiqW / 2, flaskLiquidTopY);
        appCtx.lineTo(cx + flaskLiqW / 2, flaskLiquidTopY);
        appCtx.lineTo(cx + flaskW / 2 - 2, flaskBaseY - 2);
        appCtx.lineTo(cx - flaskW / 2 + 2, flaskBaseY - 2);
        appCtx.closePath();
        appCtx.fill();

        // Stirrer vortex wave inside liquid surface
        if (stirrerSpeed > 0 && $currentPH > 0) {
          appCtx.strokeStyle = 'rgba(255, 255, 255, 0.18)';
          appCtx.lineWidth = 1.0;
          appCtx.beginPath();
          appCtx.ellipse(cx, flaskLiquidTopY + 3, flaskLiqW * 0.25, 1.5, 0, stirrerAngle * 0.05, stirrerAngle * 0.05 + Math.PI);
          appCtx.stroke();
        }

        // Update and draw ripples
        for (let i = ripples.length - 1; i >= 0; i--) {
          const r = ripples[i];
          r.r += 30 * dt;
          r.alpha -= 2.0 * dt;
          if (r.alpha <= 0) {
            ripples.splice(i, 1);
            continue;
          }
          appCtx.strokeStyle = `rgba(255, 255, 255, ${r.alpha})`;
          appCtx.lineWidth = 0.8;
          appCtx.beginPath();
          appCtx.ellipse(cx, r.y, r.r, r.r * 0.15, 0, 0, Math.PI * 2);
          appCtx.stroke();
        }

        // Draw Swirls
        for (let i = swirls.length - 1; i >= 0; i--) {
          const s = swirls[i];
          s.x += s.vx * dt;
          s.y += s.vy * dt;
          s.radius += (s.maxRadius - s.radius) * 4 * dt;
          s.alpha -= 1.4 * dt;
          
          if (s.alpha <= 0 || s.y >= flaskBaseY - 4) {
            swirls.splice(i, 1);
            continue;
          }
          
          const pct = (flaskBaseY - s.y) / (flaskBaseY - flaskTopY);
          const wAtY = flaskNeckW + (flaskW - flaskNeckW) * pct;
          const padding = s.radius + 3;
          if (s.x < cx - wAtY / 2 + padding) s.x = cx - wAtY / 2 + padding;
          if (s.x > cx + wAtY / 2 - padding) s.x = cx + wAtY / 2 - padding;

          const grad = appCtx.createRadialGradient(s.x, s.y, 0, s.x, s.y, s.radius);
          let cStr = s.color.replace('alpha', `${s.alpha}`);
          grad.addColorStop(0, cStr);
          grad.addColorStop(1, cStr.replace(`${s.alpha}`, '0'));
          
          appCtx.fillStyle = grad;
          appCtx.beginPath();
          appCtx.arc(s.x, s.y, s.radius, 0, Math.PI * 2);
          appCtx.fill();
        }

        // Flask Outline
        appCtx.strokeStyle = 'rgba(255, 255, 255, 0.3)';
        appCtx.lineWidth = 1.6;
        appCtx.beginPath();
        appCtx.moveTo(cx - flaskNeckW / 2, flaskTopY);
        appCtx.lineTo(cx + flaskNeckW / 2, flaskTopY);
        appCtx.lineTo(cx + flaskNeckW / 2, flaskTopY + 20);
        appCtx.lineTo(cx + flaskW / 2, flaskBaseY);
        appCtx.lineTo(cx - flaskW / 2, flaskBaseY);
        appCtx.lineTo(cx - flaskNeckW / 2, flaskTopY + 20);
        appCtx.closePath();
        appCtx.stroke();

        // Rim
        appCtx.beginPath();
        appCtx.ellipse(cx, flaskTopY, flaskNeckW / 2 + 2, 2.0, 0, 0, Math.PI * 2);
        appCtx.stroke();

        // Meniscus surface curve
        appCtx.strokeStyle = 'rgba(255, 255, 255, 0.16)';
        appCtx.lineWidth = 0.8;
        appCtx.beginPath();
        appCtx.ellipse(cx, flaskLiquidTopY, flaskLiqW / 2, 1.2, 0, 0, Math.PI * 2);
        appCtx.stroke();

        // 5. Magnetic Stirrer Base
        const stirrerY = flaskBaseY + 3;
        const stirGrad = appCtx.createLinearGradient(cx - 50, stirrerY, cx + 50, stirrerY);
        stirGrad.addColorStop(0, '#334155');
        stirGrad.addColorStop(0.5, '#475569');
        stirGrad.addColorStop(1, '#334155');
        appCtx.fillStyle = stirGrad;
        appCtx.beginPath();
        appCtx.roundRect(cx - 52, stirrerY, 104, 16, 4);
        appCtx.fill();
        
        // Stirrer border & highlights
        appCtx.strokeStyle = '#1e293b';
        appCtx.lineWidth = 1.5;
        appCtx.strokeRect(cx - 52, stirrerY, 104, 16);

        // Power LED indicator (grows based on stirring speed)
        appCtx.fillStyle = stirrerSpeed > 0 ? '#10b981' : '#64748b'; // Green when spinning
        appCtx.beginPath();
        appCtx.arc(cx - 40, stirrerY + 8, 2.5, 0, Math.PI * 2);
        appCtx.fill();

        // Stirrer capsule inside flask
        const barWidth = 24 * Math.cos(stirrerAngle * 0.05); // Spin animation
        appCtx.fillStyle = '#f8fafc';
        appCtx.strokeStyle = '#cbd5e1';
        appCtx.lineWidth = 0.8;
        appCtx.beginPath();
        appCtx.roundRect(cx - barWidth / 2, flaskBaseY - 8, barWidth, 4.5, 2.2);
        appCtx.fill();
        appCtx.stroke();

        // pH probe electrode inside flask
        const probeX = cx - flaskNeckW / 2 + 5;
        const probeY1 = flaskTopY - 12;
        const probeY2 = flaskLiquidTopY + 22;
        
        // Probe rod
        appCtx.fillStyle = '#1e293b';
        appCtx.fillRect(probeX - 2.5, probeY1, 5, probeY2 - probeY1);
        appCtx.strokeStyle = '#475569';
        appCtx.lineWidth = 0.8;
        appCtx.strokeRect(probeX - 2.5, probeY1, 5, probeY2 - probeY1);

        // Glass bulb tip
        const probeBulbGrad = appCtx.createRadialGradient(probeX, probeY2, 0, probeX, probeY2, 4);
        probeBulbGrad.addColorStop(0, 'rgba(56, 189, 248, 0.8)');
        probeBulbGrad.addColorStop(1, 'rgba(14, 165, 233, 0.4)');
        appCtx.fillStyle = probeBulbGrad;
        appCtx.beginPath();
        appCtx.arc(probeX, probeY2, 4.0, 0, Math.PI * 2);
        appCtx.fill();
        appCtx.strokeStyle = 'rgba(255, 255, 255, 0.5)';
        appCtx.lineWidth = 0.6;
        appCtx.stroke();

        // Probe wire leading out
        appCtx.strokeStyle = '#475569';
        appCtx.lineWidth = 1.0;
        appCtx.beginPath();
        appCtx.moveTo(probeX, probeY1);
        appCtx.bezierCurveTo(probeX - 15, probeY1 - 25, cx - 35, 95, cx - 35, 75);
        appCtx.stroke();

        // 6. Magnifying glass zoom of meniscus
        drawZoomView(appCtx, appW, appH);
      }

      // Render Graph Canvas
      const graphCtx = graphCanvas?.getContext('2d');
      if (graphCtx) {
        graphCtx.setTransform(1, 0, 0, 1, 0, 0);
        graphCtx.clearRect(0, 0, graphCanvas.width, graphCanvas.height);
        graphCtx.scale(dpr, dpr);

        const gW = graphCanvas.width / dpr;
        const gH = graphCanvas.height / dpr;

        const graphX = 40;
        const graphY = 20;
        const graphW = gW - graphX - 20;
        const graphH = gH - graphY - 35;

        // Dark Graph frame background
        graphCtx.fillStyle = '#020617';
        graphCtx.fillRect(graphX, graphY, graphW, graphH);
        graphCtx.strokeStyle = '#1e293b';
        graphCtx.lineWidth = 1.5;
        graphCtx.strokeRect(graphX, graphY, graphW, graphH);

        // Draw indicator transition band
        drawIndicatorBand(graphCtx, graphX, graphY, graphW, graphH);

        // Graph Grid
        graphCtx.strokeStyle = 'rgba(51, 65, 85, 0.25)';
        graphCtx.lineWidth = 0.8;
        // Volume X grid
        for (let v = 10; v < 50; v += 10) {
          const gx = graphX + (v / 50.0) * graphW;
          graphCtx.beginPath();
          graphCtx.moveTo(gx, graphY);
          graphCtx.lineTo(gx, graphY + graphH);
          graphCtx.stroke();
        }
        // pH Y grid
        for (let p = 2; p <= 12; p += 2) {
          const gy = graphY + graphH - (p / 14.0) * graphH;
          graphCtx.beginPath();
          graphCtx.moveTo(graphX, gy);
          graphCtx.lineTo(graphX + graphW, gy);
          graphCtx.stroke();
        }

        // Ticks and Labels
        graphCtx.fillStyle = '#94a3b8';
        graphCtx.font = '8px monospace';
        graphCtx.textAlign = 'right';
        graphCtx.textBaseline = 'middle';
        // Y-axis pH labels
        for (let p = 0; p <= 14; p += 2) {
          const gy = graphY + graphH - (p / 14.0) * graphH;
          graphCtx.fillText(`${p}`, graphX - 6, gy);
        }
        // X-axis Volume labels
        graphCtx.textAlign = 'center';
        graphCtx.textBaseline = 'top';
        for (let v = 0; v <= 50; v += 10) {
          const gx = graphX + (v / 50.0) * graphW;
          graphCtx.fillText(`${v}`, gx, graphY + graphH + 6);
        }

        // Axis Titles
        graphCtx.fillStyle = '#64748b';
        graphCtx.font = 'bold 9px sans-serif';
        graphCtx.textAlign = 'center';
        graphCtx.fillText('Volume NaOH Added (mL)', graphX + graphW / 2, graphY + graphH + 20);

        graphCtx.save();
        graphCtx.translate(graphX - 22, graphY + graphH / 2);
        graphCtx.rotate(-Math.PI / 2);
        graphCtx.fillText('pH Value', 0, 0);
        graphCtx.restore();

        // Draw titration curve cyan line
        const M1 = $titrationState.titrantConc;
        const V2 = $titrationState.analyteVolume;
        const M2 = $titrationState.analyteConc;

        graphCtx.strokeStyle = '#06b6d4'; // Cyan neon curve
        graphCtx.lineWidth = 1.8;
        graphCtx.beginPath();
        for (let v = 0; v <= 50.0; v += 0.25) {
          const phVal = calculatePH(v, M1, V2, M2);
          const gx = graphX + (v / 50.0) * graphW;
          const gy = graphY + graphH - (phVal / 14.0) * graphH;
          if (v === 0) {
            graphCtx.moveTo(gx, gy);
          } else {
            graphCtx.lineTo(gx, gy);
          }
        }
        graphCtx.stroke();

        // Draw Equivalence Point line overlay (Theoretical Equivalence point)
        const vEq = V2 * (M2 / M1);
        if (vEq <= 50.0) {
          const eqX = graphX + (vEq / 50.0) * graphW;
          const eqY = graphY + graphH - (7.0 / 14.0) * graphH;
          
          graphCtx.strokeStyle = 'rgba(239, 68, 68, 0.35)'; // Red dashed line
          graphCtx.lineWidth = 0.8;
          graphCtx.setLineDash([3, 3]);
          graphCtx.beginPath();
          graphCtx.moveTo(eqX, graphY);
          graphCtx.lineTo(eqX, graphY + graphH);
          graphCtx.stroke();
          graphCtx.setLineDash([]);

          // Equivalence Point dot
          graphCtx.fillStyle = '#ef4444';
          graphCtx.beginPath();
          graphCtx.arc(eqX, eqY, 3.5, 0, Math.PI * 2);
          graphCtx.fill();
          graphCtx.strokeStyle = '#ffffff';
          graphCtx.lineWidth = 0.8;
          graphCtx.stroke();

          // Label
          graphCtx.fillStyle = 'rgba(239, 68, 68, 0.7)';
          graphCtx.font = 'bold 7px sans-serif';
          graphCtx.textAlign = 'left';
          graphCtx.fillText(`Eq Point (${vEq.toFixed(2)} mL)`, eqX + 5, graphY + 10);
        }

        // Draw active volume/pH point marker
        const curX = graphX + ($titrationState.addedVolume / 50.0) * graphW;
        const curY = graphY + graphH - ($currentPH / 14.0) * graphH;

        // Dashed lines from current point to axes
        graphCtx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
        graphCtx.lineWidth = 0.8;
        graphCtx.setLineDash([2, 2]);
        
        graphCtx.beginPath();
        graphCtx.moveTo(curX, curY);
        graphCtx.lineTo(curX, graphY + graphH);
        graphCtx.stroke();
        
        graphCtx.beginPath();
        graphCtx.moveTo(curX, curY);
        graphCtx.lineTo(graphX, curY);
        graphCtx.stroke();
        graphCtx.setLineDash([]);

        // Pulsing active marker
        graphCtx.save();
        graphCtx.shadowColor = '#06b6d4';
        graphCtx.shadowBlur = 8;
        graphCtx.fillStyle = '#22d3ee';
        graphCtx.beginPath();
        graphCtx.arc(curX, curY, 4.5, 0, Math.PI * 2);
        graphCtx.fill();
        graphCtx.restore();

        graphCtx.strokeStyle = '#ffffff';
        graphCtx.lineWidth = 1.0;
        graphCtx.beginPath();
        graphCtx.arc(curX, curY, 4.5, 0, Math.PI * 2);
        graphCtx.stroke();

        // Draw hover tooltip if active
        drawHoverTooltip(graphCtx, graphX, graphY, graphW, graphH, M1, V2, M2);
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

<div class="flex flex-col lg:flex-row h-full w-full bg-slate-950 text-slate-100 overflow-hidden font-sans select-none">
  <!-- Left Column: Chemistry Lab Apparatus -->
  <div class="relative w-full lg:w-[280px] xl:w-[320px] h-[200px] sm:h-[280px] lg:h-full border-b lg:border-b-0 lg:border-r border-slate-800/80 flex flex-col bg-slate-900/10">

    
    <div class="flex-1 w-full relative min-h-0">
      <canvas 
        bind:this={apparatusCanvas} 
        on:mousemove={handleApparatusMouseMove}
        on:mousedown={handleApparatusClick}
        class="w-full h-full block"
      ></canvas>
    </div>
  </div>

  <!-- Right Column: Analytical Dashboard -->
  <div class="flex-1 h-full flex flex-col min-w-0 bg-slate-950/20">
    <!-- Graph Panel -->
    <div class="flex-1 relative min-h-[140px] sm:min-h-[180px] lg:min-h-[220px] p-4 pb-0">
      <canvas 
        bind:this={graphCanvas} 
        on:mousemove={handleGraphMouseMove}
        on:mouseleave={handleGraphMouseLeave}
        class="w-full h-full block bg-slate-950/40 rounded-lg border border-slate-800/40"
      ></canvas>
    </div>
  </div>
</div>
