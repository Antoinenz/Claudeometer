import { useEffect, useRef } from "react";
import confetti from "canvas-confetti";

const COLORS = ["#f59e0b", "#10b981", "#3b82f6", "#f43f5e", "#a78bfa", "#fb923c"];
const BURST_MS = 1000;

export default function Confetti({ onDone }: { onDone: () => void }) {
  const onDoneRef = useRef(onDone);
  onDoneRef.current = onDone;

  useEffect(() => {
    const canvas = document.createElement("canvas");
    canvas.style.cssText = "position:fixed;top:0;left:0;width:100vw;height:100vh;pointer-events:none;z-index:99999";
    document.body.appendChild(canvas);

    const fire = confetti.create(canvas, { resize: true, useWorker: false });
    const end = Date.now() + BURST_MS;
    let frameId: number;
    let lastPromises: Promise<unknown>[] = [];
    let cancelled = false;

    function tick() {
      lastPromises = [
        fire({ particleCount: 4, angle: 60,  spread: 70, origin: { x: 0, y: 0.75 }, colors: COLORS }) ?? Promise.resolve(),
        fire({ particleCount: 4, angle: 120, spread: 70, origin: { x: 1, y: 0.75 }, colors: COLORS }) ?? Promise.resolve(),
      ];
      if (Date.now() < end) {
        frameId = requestAnimationFrame(tick);
      } else {
        // Burst over — wait for remaining particles to drift off screen, then clean up.
        Promise.all(lastPromises).then(() => {
          if (!cancelled) {
            canvas.remove();
            onDoneRef.current();
          }
        });
      }
    }

    frameId = requestAnimationFrame(tick);
    return () => {
      cancelled = true;
      cancelAnimationFrame(frameId);
      fire.reset();
      canvas.remove();
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return null;
}
