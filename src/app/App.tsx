import "./theme.css";
import "./layout.css";

/** The app shell: a header and one main region the features compose into. */
export function App() {
  return (
    <>
      <header className="app-header">
        <h1>DW4 Very Hard Plus</h1>
        <p className="app-subtitle">
          Turn your own clean Digimon World 4 (USA) disc into a much harder one.
        </p>
      </header>
      <main className="app-main layout-one-col" />
    </>
  );
}
