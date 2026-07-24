import {createFileRoute} from '@tanstack/react-router'
import {createServerFn} from '@tanstack/react-start'
import {LandingPage, PROMPT_EXAMPLE} from '@/components/landing/landing-page'

const highlightSample = createServerFn({method: 'GET'}).handler(async () => {
  const {highlightNapl} = await import('@/lib/highlight')
  return highlightNapl(PROMPT_EXAMPLE)
})

export const Route = createFileRoute('/')({
  loader: () => highlightSample(),
  component: Home,
})

function Home() {
  const sampleHtml = Route.useLoaderData()
  return <LandingPage sampleHtml={sampleHtml} />
}
