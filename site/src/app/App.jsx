import { SiteFooter } from "../components/SiteFooter";
import { SiteHeader } from "../components/SiteHeader";
import { DocsPage } from "../pages/DocsPage";
import { DownloadPage } from "../pages/DownloadPage";
import { HomePage } from "../pages/HomePage";
import { ModelDetailPage } from "../pages/ModelDetailPage";
import { ModelsPage } from "../pages/ModelsPage";
import { NotFoundPage } from "../pages/NotFoundPage";
import { useLocationRoute } from "./router";

export default function App() {
  const location = useLocationRoute();
  const { route } = location;

  let page;
  switch (route.name) {
    case "home":
      page = <HomePage />;
      break;
    case "models":
      page = <ModelsPage location={location} />;
      break;
    case "model":
      page = <ModelDetailPage {...route.params} />;
      break;
    case "docs":
      page = <DocsPage slug={route.params.slug} />;
      break;
    case "download":
      page = <DownloadPage />;
      break;
    default:
      page = <NotFoundPage />;
  }

  return (
    <div className="site">
      <SiteHeader pathname={location.pathname} />
      {page}
      <SiteFooter />
    </div>
  );
}
