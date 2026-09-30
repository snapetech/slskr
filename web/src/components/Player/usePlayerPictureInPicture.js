import { useEffect, useRef } from 'react';
import { getFrequencyBars } from './SpectrumAnalyzer';
import { resumeAudioGraph } from './audioGraph';

export default function usePlayerPictureInPicture({ audioRef, mountedRef }) {
  const pipRef = useRef({ raf: null, win: null });
  const pipRequestIdRef = useRef(0);

  useEffect(() => () => {
    pipRequestIdRef.current += 1;
    const { raf, win } = pipRef.current;
    if (raf !== null && win?.cancelAnimationFrame) {
      win.cancelAnimationFrame(raf);
    }
    if (win && !win.closed) {
      win.close();
    }
    pipRef.current = { raf: null, win: null };
  }, []);

  const openPictureInPicture = async () => {
    if (!audioRef.current || !window.documentPictureInPicture) return;

    const requestId = ++pipRequestIdRef.current;
    const isCurrentRequest = () =>
      mountedRef.current && pipRequestIdRef.current === requestId;

    try {
      const graph = await resumeAudioGraph(audioRef.current);
      if (!graph || !isCurrentRequest()) return;

      const pipWindow = await window.documentPictureInPicture.requestWindow({
        height: 220,
        width: 360,
      });
      if (!isCurrentRequest()) {
        if (!pipWindow.closed) pipWindow.close();
        return;
      }
      pipWindow.document.body.style.margin = '0';
      pipWindow.document.body.style.background = '#050608';
      const canvas = pipWindow.document.createElement('canvas');
      canvas.style.height = '100%';
      canvas.style.width = '100%';
      pipWindow.document.body.appendChild(canvas);
      pipRef.current.win = pipWindow;

      const draw = () => {
        if (pipWindow.closed || !isCurrentRequest()) return;
        const width = Math.max(1, pipWindow.innerWidth);
        const height = Math.max(1, pipWindow.innerHeight);
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        const data = new Uint8Array(graph.analyser.frequencyBinCount);
        graph.analyser.getByteFrequencyData(data);
        ctx.fillStyle = '#050608';
        ctx.fillRect(0, 0, width, height);
        const barCount = Math.min(72, Math.max(16, Math.floor(width / 7)));
        const bars = getFrequencyBars(data, barCount);
        const barWidth = width / bars.length;
        bars.forEach((value, index) => {
          const barHeight = (value / 255) * height;
          ctx.fillStyle = `hsl(${130 - (index / bars.length) * 100}, 75%, 54%)`;
          ctx.fillRect(
            index * barWidth,
            height - barHeight,
            Math.max(1, barWidth - 1),
            barHeight,
          );
        });
        pipRef.current.raf = pipWindow.requestAnimationFrame(draw);
      };

      draw();
    } catch {
      // Browser support and permission can change while the player unmounts.
    }
  };

  return openPictureInPicture;
}
