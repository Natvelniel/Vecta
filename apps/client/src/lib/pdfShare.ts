/**
 * Shares a generated ATS PDF natively on Android/iOS via Web Share API,
 * falling back cleanly to a browser Blob download on Desktop.
 */
export async function shareOrDownloadPdf(
  fileName: string,
  pdfBase64: string,
): Promise<"shared" | "downloaded"> {
  const cleanName = fileName.trim() || "Vecta_CV_ATS.pdf";
  const binary = atob(pdfBase64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  const blob = new Blob([bytes], { type: "application/pdf" });

  if (typeof navigator !== "undefined" && typeof navigator.share === "function") {
    try {
      const file = new File([blob], cleanName, { type: "application/pdf" });
      if (typeof navigator.canShare !== "function" || navigator.canShare({ files: [file] })) {
        await navigator.share({
          files: [file],
          title: cleanName,
        });
        return "shared";
      }
    } catch (err) {
      if (err instanceof Error && err.name === "AbortError") {
        return "shared";
      }
    }
  }

  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = cleanName;
  anchor.style.display = "none";
  document.body.appendChild(anchor);
  anchor.click();
  setTimeout(() => {
    document.body.removeChild(anchor);
    URL.revokeObjectURL(url);
  }, 1500);
  return "downloaded";
}
