import { Shell } from "./components/Shell";
import { StoreProvider } from "./state/store";

export default function App() {
  return (
    <StoreProvider>
      <Shell />
    </StoreProvider>
  );
}
