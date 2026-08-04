import { useEffect, useRef } from "react";

export function useScrollProgress(onProgress, disabled = false) {
  const sectionRef = useRef(null);
  const callbackRef = useRef(onProgress);

  useEffect(() => {
    callbackRef.current = onProgress;
  }, [onProgress]);

  useEffect(() => {
    const section = sectionRef.current;
    if (!section || disabled) return undefined;

    let frame = 0;

    const update = () => {
      frame = 0;
      const bounds = section.getBoundingClientRect();
      const scrollRange = Math.max(section.offsetHeight - window.innerHeight, 1);
      const travelled = Math.min(scrollRange, Math.max(0, -bounds.top));
      callbackRef.current(travelled / scrollRange, section);
    };

    const requestUpdate = () => {
      if (frame) return;
      frame = window.requestAnimationFrame(update);
    };

    update();
    window.addEventListener("scroll", requestUpdate, { passive: true });
    window.addEventListener("resize", requestUpdate);

    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener("scroll", requestUpdate);
      window.removeEventListener("resize", requestUpdate);
    };
  }, [disabled]);

  return sectionRef;
}

export function useReducedMotion() {
  const reducedRef = useRef(false);

  useEffect(() => {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    reducedRef.current = query.matches;
    const update = () => {
      reducedRef.current = query.matches;
    };
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);

  return reducedRef;
}
