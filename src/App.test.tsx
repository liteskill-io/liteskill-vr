import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import App from "./App";

describe("App", () => {
  it("renders the dashboard", () => {
    render(<App />);
    // Match the dashboard empty-state wordmark specifically (all-caps),
    // since the sidebar brand also renders the name in mixed case.
    expect(screen.getByText("LITESKILL")).toBeInTheDocument();
  });
});
