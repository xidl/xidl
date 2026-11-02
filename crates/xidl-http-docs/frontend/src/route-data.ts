import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import publication from '../xidl-site.json';

export const onRequest = defineRouteMiddleware(context => {
  const playground = publication.playgrounds.some(
    reference =>
      context.url.pathname === publication.base + reference.pathname.slice(1),
  );
  if (playground) context.locals.starlightRoute.hasSidebar = true;
});
