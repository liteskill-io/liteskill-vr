import { browser, expect } from "@wdio/globals";

describe("LiteSkill VR", () => {
  it("should launch and show the heading", async () => {
    const heading = await browser.$("h1");
    await expect(heading).toHaveText("LiteSkill VR");
  });

  it("should have the correct window title", async () => {
    const title = await browser.getTitle();
    expect(title).toBe("LiteSkill VR");
  });
});
