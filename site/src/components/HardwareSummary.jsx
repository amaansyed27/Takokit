export function hardwareLabel(hardware) {
  if (!hardware) return "Not declared";
  if (hardware.gpuRequired) {
    return hardware.minVram ? `GPU required · ${hardware.minVram} VRAM` : "GPU required";
  }
  if (hardware.cpu && !hardware.gpu) return "CPU friendly";
  if (hardware.cpu && hardware.gpu) {
    return hardware.minVram ? `CPU supported · GPU ${hardware.minVram} VRAM` : "CPU and GPU supported";
  }
  if (hardware.gpu) return "GPU supported";
  return "Not declared";
}

export function HardwareSummary({ hardware, detailed = false }) {
  if (!detailed) return <span>{hardwareLabel(hardware)}</span>;
  return (
    <dl className="hardware-grid">
      <div><dt>CPU</dt><dd>{hardware?.cpu ? "Supported" : "Not supported"}</dd></div>
      <div><dt>GPU</dt><dd>{hardware?.gpu ? (hardware.gpuRequired ? "Required" : "Supported") : "Not supported"}</dd></div>
      <div><dt>Minimum RAM</dt><dd>{hardware?.minRam || "Not declared"}</dd></div>
      <div><dt>Minimum VRAM</dt><dd>{hardware?.minVram || "Not declared"}</dd></div>
    </dl>
  );
}
