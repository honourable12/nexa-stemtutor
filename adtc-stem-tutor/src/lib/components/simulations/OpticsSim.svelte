<script lang="ts">
  import { onMount } from 'svelte';
  import { labState } from '$lib/stores/labStore';

  let canvas: HTMLCanvasElement;
  let animationFrameId: number;

  onMount(() => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const resize = () => {
      canvas.width = canvas.parentElement?.clientWidth || 400;
      canvas.height = canvas.parentElement?.clientHeight || 400;
    };
    window.addEventListener('resize', resize);
    resize();

    const loop = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const cx = canvas.width / 2;
      const cy = canvas.height / 2;

      // Parameters
      const f = $labState.opticsFocalLength;
      const doVal = $labState.opticsObjectDistance;
      const ho = $labState.opticsObjectHeight;
      const isConcave = $labState.opticsLensType === 'Concave';

      // Scaling factors
      const scaleX = 4.0; // scale from cm to pixels
      const scaleY = 3.0;

      const pxF = f * scaleX;
      const pxDo = doVal * scaleX;
      const pxHo = ho * scaleY;

      // Compute Thin Lens Equation: 1/f = 1/do + 1/di
      // di = (f * do) / (do - f)
      let diVal = 0;
      let realImage = true;
      let infinite = false;

      if (isConcave) {
        // Concave lens has negative focal length: f_eq = -f
        // di = (-f * do) / (do + f)
        diVal = (-f * doVal) / (doVal + f);
        realImage = false;
      } else {
        if (Math.abs(doVal - f) < 0.05) {
          infinite = true;
        } else {
          diVal = (f * doVal) / (doVal - f);
          realImage = diVal > 0;
        }
      }

      const pxDi = diVal * scaleX;
      // hi = - (di/do) * ho
      // Since canvas y-axis points down:
      // objY = cy - pxHo (above axis for positive pxHo)
      // pxHi = - (diVal / doVal) * pxHo
      // imgY = cy - pxHi (so if pxHi is negative [real inverted], imgY is cy + |pxHi| [below axis])
      const pxHi = - (diVal / doVal) * pxHo; 

      const imgX = cx + pxDi;
      const imgY = cy - pxHi;

      // --- 1. Draw Optical Axis ---
      ctx.strokeStyle = '#334155';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(10, cy);
      ctx.lineTo(canvas.width - 10, cy);
      ctx.stroke();

      // --- 2. Draw Focal Points (F) and Double Focal Points (2F) ---
      ctx.fillStyle = '#64748b';
      ctx.font = '10px monospace';
      ctx.textAlign = 'center';

      // Left focal point
      ctx.beginPath();
      ctx.arc(cx - pxF, cy, 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillText("F", cx - pxF, cy + 15);

      // Right focal point
      ctx.beginPath();
      ctx.arc(cx + pxF, cy, 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillText("F", cx + pxF, cy + 15);

      // --- 3. Draw Convex or Concave Lens ---
      ctx.strokeStyle = '#38bdf8'; // Sky blue lens
      ctx.fillStyle = 'rgba(56, 189, 248, 0.1)';
      ctx.lineWidth = 3;
      ctx.beginPath();
      if (isConcave) {
        // Draw double concave hourglass shape
        ctx.moveTo(cx - 15, cy - 100);
        ctx.lineTo(cx + 15, cy - 100);
        ctx.quadraticCurveTo(cx + 5, cy, cx + 15, cy + 100);
        ctx.lineTo(cx - 15, cy + 100);
        ctx.quadraticCurveTo(cx - 5, cy, cx - 15, cy - 100);
      } else {
        // Draw double convex shape
        ctx.moveTo(cx, cy - 100);
        ctx.quadraticCurveTo(cx + 15, cy, cx, cy + 100);
        ctx.quadraticCurveTo(cx - 15, cy, cx, cy - 100);
      }
      ctx.fill();
      ctx.stroke();

      // --- 4. Draw Object Arrow (Green) ---
      const objX = cx - pxDo;
      const objY = cy - pxHo;

      ctx.strokeStyle = '#4ade80'; // Green Object
      ctx.lineWidth = 4;
      ctx.beginPath();
      ctx.moveTo(objX, cy);
      ctx.lineTo(objX, objY);
      ctx.stroke();
      // Arrow head
      ctx.fillStyle = '#4ade80';
      ctx.beginPath();
      ctx.moveTo(objX - 6, objY + (pxHo > 0 ? 8 : -8));
      ctx.lineTo(objX, objY);
      ctx.lineTo(objX + 6, objY + (pxHo > 0 ? 8 : -8));
      ctx.fill();

      ctx.fillStyle = '#4ade80';
      ctx.fillText(`Object (do=${doVal}cm)`, objX, cy - pxHo - 15);

      // --- 5. Draw Rays & Image ---
      if (infinite) {
        // Parallel exiting rays for convex lens when object is at F
        ctx.strokeStyle = 'rgba(234, 179, 8, 0.65)'; // Yellow light rays
        ctx.lineWidth = 1.5;

        // Parallel Ray 1
        ctx.beginPath();
        ctx.moveTo(objX, objY);
        ctx.lineTo(cx, objY);
        ctx.lineTo(canvas.width - 10, objY + (canvas.width - cx) * (pxHo / pxF));
        ctx.stroke();

        // Chief Ray 2
        ctx.beginPath();
        ctx.moveTo(objX, objY);
        ctx.lineTo(cx, cy);
        ctx.lineTo(canvas.width - 10, cy + (canvas.width - cx) * (pxHo / pxDo));
        ctx.stroke();

        ctx.fillStyle = '#ef4444';
        ctx.fillText("Image at Infinity", cx + 120, cy + 40);
      } else {
        // Draw Image Arrow (Red)
        ctx.strokeStyle = '#f87171'; // Red Image
        ctx.lineWidth = 4;
        ctx.beginPath();
        ctx.moveTo(imgX, cy);
        ctx.lineTo(imgX, imgY);
        ctx.stroke();
        
        // Arrow head pointing to imgY
        ctx.fillStyle = '#f87171';
        ctx.beginPath();
        const arrowDir = imgY < cy ? 8 : -8;
        ctx.moveTo(imgX - 6, imgY + arrowDir);
        ctx.lineTo(imgX, imgY);
        ctx.lineTo(imgX + 6, imgY + arrowDir);
        ctx.fill();

        ctx.fillStyle = '#f87171';
        ctx.fillText(`Image (di=${diVal.toFixed(1)}cm)`, imgX, imgY + (imgY < cy ? -15 : 20));

        // Draw Ray Tracing
        ctx.lineWidth = 1.5;

        if (isConcave) {
          // --- Concave (Diverging) Lens Ray Tracing ---

          // Ray 1: Parallel to axis, then diverges from near focal point (F_near = cx - pxF)
          const slope1 = (objY - cy) / pxF;
          const exitY1 = objY + (canvas.width - cx) * slope1;

          // Real ray (solid)
          ctx.strokeStyle = 'rgba(245, 158, 11, 0.7)'; // Amber
          ctx.beginPath();
          ctx.moveTo(objX, objY);
          ctx.lineTo(cx, objY);
          ctx.lineTo(canvas.width - 10, exitY1);
          ctx.stroke();

          // Virtual backtracked ray (dashed)
          ctx.strokeStyle = 'rgba(245, 158, 11, 0.35)';
          ctx.setLineDash([4, 4]);
          ctx.beginPath();
          ctx.moveTo(cx, objY);
          ctx.lineTo(cx - pxF, cy);
          ctx.stroke();
          ctx.setLineDash([]);

          // Ray 2: Chief Ray (straight through center)
          const slope2 = (cy - objY) / pxDo;
          const exitY2 = cy + (canvas.width - cx) * slope2;

          ctx.strokeStyle = 'rgba(16, 185, 129, 0.7)'; // Emerald
          ctx.beginPath();
          ctx.moveTo(objX, objY);
          ctx.lineTo(cx, cy);
          ctx.lineTo(canvas.width - 10, exitY2);
          ctx.stroke();

          // Ray 3: Directed towards far focal point (F_far = cx + pxF), emerges parallel
          ctx.strokeStyle = 'rgba(99, 102, 241, 0.7)'; // Indigo
          ctx.beginPath();
          ctx.moveTo(objX, objY);
          ctx.lineTo(cx, imgY);
          ctx.lineTo(canvas.width - 10, imgY);
          ctx.stroke();

          // Virtual backtracked ray (dashed)
          ctx.strokeStyle = 'rgba(99, 102, 241, 0.35)';
          ctx.setLineDash([4, 4]);
          ctx.beginPath();
          ctx.moveTo(cx, imgY);
          ctx.lineTo(imgX, imgY);
          ctx.stroke();
          ctx.setLineDash([]);

        } else {
          // --- Convex (Converging) Lens Ray Tracing ---

          // Ray 1: Parallel to axis, then bends through far focal point F_far (cx + pxF, cy)
          ctx.strokeStyle = 'rgba(245, 158, 11, 0.7)'; // Amber
          ctx.beginPath();
          ctx.moveTo(objX, objY);
          ctx.lineTo(cx, objY);
          if (realImage) {
            ctx.lineTo(imgX, imgY);
            const slope = (imgY - objY) / (imgX - cx);
            ctx.lineTo(canvas.width - 10, objY + slope * (canvas.width - 10 - cx));
            ctx.stroke();
          } else {
            const slope = (cy - objY) / pxF;
            const exitY = objY + (canvas.width - cx) * slope;
            ctx.lineTo(canvas.width - 10, exitY);
            ctx.stroke();

            // Backtrack to virtual image
            ctx.strokeStyle = 'rgba(245, 158, 11, 0.35)';
            ctx.setLineDash([4, 4]);
            ctx.beginPath();
            ctx.moveTo(cx, objY);
            ctx.lineTo(imgX, imgY);
            ctx.stroke();
            ctx.setLineDash([]);
          }

          // Ray 2: Chief Ray (straight through center)
          ctx.strokeStyle = 'rgba(16, 185, 129, 0.7)'; // Emerald
          ctx.beginPath();
          ctx.moveTo(objX, objY);
          ctx.lineTo(cx, cy);
          const slope2 = (cy - objY) / pxDo;
          ctx.lineTo(canvas.width - 10, cy + (canvas.width - 10 - cx) * slope2);
          ctx.stroke();

          // Ray 3: Through near focal point F_near (or aligned with it), emerges parallel
          ctx.strokeStyle = 'rgba(99, 102, 241, 0.7)'; // Indigo
          ctx.beginPath();
          ctx.moveTo(objX, objY);
          ctx.lineTo(cx, imgY);
          ctx.lineTo(canvas.width - 10, imgY);
          ctx.stroke();

          if (!realImage) {
            // Virtual image backtrack
            ctx.strokeStyle = 'rgba(99, 102, 241, 0.35)';
            ctx.setLineDash([4, 4]);
            ctx.beginPath();
            ctx.moveTo(cx, imgY);
            ctx.lineTo(imgX, imgY);
            ctx.stroke();
            ctx.setLineDash([]);
          }
        }
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
