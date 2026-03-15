import { test, expect } from '@playwright/test';

test.describe('mini-bpm Tauri App UI', () => {
  test('should load the BPMN Editor container', async ({ page }) => {
    // Navigate to the Vite dev server URL
    await page.goto('/');

    // Wait for the app component to be mounted
    const canvasContainer = page.locator('.canvas');
    await expect(canvasContainer).toBeVisible({ timeout: 10000 });

    // Verify there is a BPMN logo or element
    // This is just a basic sanity check that it injected the BPMN modeler
    const bjsContainer = page.locator('.bjs-container');
    await expect(bjsContainer).toBeVisible({ timeout: 10000 });
  });

  test('should verify the properties panel is visible', async ({ page }) => {
    await page.goto('/');

    // Wait for the properties panel to mount
    const propertiesPanel = page.locator('.properties-panel-parent');
    await expect(propertiesPanel).toBeVisible({ timeout: 10000 });
  });
});
