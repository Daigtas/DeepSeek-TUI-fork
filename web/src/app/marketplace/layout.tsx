import { redirect } from 'next/navigation'
import { auth } from '@/lib/auth'
import { headers } from 'next/headers'
import ZoneShell from '@/components/composer/ZoneShell'
import Toaster from '@/components/ui/Toaster'
import type { Metadata } from 'next'

const ALLOWED_ROLES = ['DEVELOPER', 'MANAGER', 'ADMIN', 'OWNER'] as const

export const metadata: Metadata = {
  title: { default: 'Plugin Marketplace — ForgeHub', template: '%s | ForgeHub' },
}

export default async function MarketplaceLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const session = await auth.api.getSession({ headers: await headers() })
  if (!session) redirect('/sign-in')

  const role = (session.user as { role?: string }).role
  if (!ALLOWED_ROLES.includes(role as typeof ALLOWED_ROLES[number])) {
    redirect('/client')
  }

  return (
    <ZoneShell zone="developer">
      {children}
      <Toaster />
    </ZoneShell>
  )
}
