export type OcrStatus = "idle" | "loading" | "success" | "empty" | "error";

export type OcrResponse = {
  text: string;
  previewBase64: string;
  imageWidth: number;
  imageHeight: number;
  error: string | null;
};
