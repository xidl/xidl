import { Playground } from '@/components/playground';

export const dynamic = 'force-static';

export default function Page() {
  return (
    <main className="min-h-screen">
      <Playground />
    </main>
  );
}
