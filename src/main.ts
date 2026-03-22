import * as PIXI from "pixi.js";
import { Live2DModel } from "pixi-live2d-display";

// Expose PIXI globally for pixi-live2d-display compatibility
(window as any).PIXI = PIXI;
// @ts-ignore - version mismatch between pixi v7 and pixi-live2d-display's pixi v6 types
Live2DModel.registerTicker(PIXI.Ticker);

const canvas = document.getElementById("live2d-canvas") as HTMLCanvasElement;

const app = new PIXI.Application({
  view: canvas,
  backgroundAlpha: 0,
  resizeTo: window,
  antialias: true,
});

async function loadModel() {
  const modelPath = "./model/atri.model3.json";

  try {
    const model = await Live2DModel.from(modelPath, {
      autoInteract: false,
    });

    app.stage.addChild(model as unknown as PIXI.DisplayObject);

    const scale = Math.min(
      canvas.width / model.width,
      canvas.height / model.height
    ) * 0.8;
    model.scale.set(scale);
    model.x = (canvas.width - model.width * scale) / 2;
    model.y = (canvas.height - model.height * scale) / 2;

    model.on("hit", (hitAreas: string[]) => {
      if (hitAreas.includes("body") || hitAreas.includes("Body")) {
        model.motion("tap_body");
      } else if (hitAreas.includes("head") || hitAreas.includes("Head")) {
        model.motion("flick_head");
      }
    });

    console.log("Live2D model loaded");
  } catch (e) {
    console.warn("No model found at", modelPath);
    const text = new PIXI.Text("Place ATRI model\nin public/model/", {
      fontFamily: "Arial",
      fontSize: 18,
      fill: 0xffffff,
      align: "center",
    });
    text.anchor.set(0.5);
    text.x = canvas.width / 2;
    text.y = canvas.height / 2;
    app.stage.addChild(text);
  }
}

async function setupDrag() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const appWindow = getCurrentWindow();

  canvas.addEventListener("mousedown", (e) => {
    if (e.button === 0) {
      appWindow.startDragging();
    }
  });

  canvas.addEventListener("contextmenu", (e) => {
    e.preventDefault();
  });
}

loadModel();
setupDrag();
