import { Toaster } from "sonner";

import { TooltipProvider } from "@/components/ui/tooltip";

import { Shell } from "./components/Shell";
import { StoreProvider } from "./state/store";

export default function App() {
  return (
    <TooltipProvider>
      <StoreProvider>
        <Shell />
        <Toaster position="top-right" richColors />
      </StoreProvider>
    </TooltipProvider>
  );
}
