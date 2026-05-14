/**
 * Plugin Marketplace Browse Page — Phase 10 PLUGIN-01.
 *
 * Public browse page at /marketplace/plugins with search, category filter,
 * and install/purchase actions.
 */

'use client'

import { useState, useEffect, useCallback } from 'react'
import { useRouter } from 'next/navigation'
import { Search, Filter, Star, Download, ShoppingCart, ArrowRight, Package } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { PageHeader } from '@/components/ui/molecules'

interface Plugin {
  id: string
  name: string
  description: string
  version: string
  author: string
  category: string
  licenseType: string
  price: number
  tags: string[]
  icon?: string
  downloads: number
  averageRating: number
  screenshots: string[]
  _count: { reviews: number }
}

export default function PluginMarketplacePage() {
  const router = useRouter()
  const [plugins, setPlugins] = useState<Plugin[]>([])
  const [loading, setLoading] = useState(true)
  const [search, setSearch] = useState('')
  const [category, setCategory] = useState('')
  const [page, setPage] = useState(1)
  const [total, setTotal] = useState(0)
  const [categories, setCategories] = useState<string[]>([])
  const [installing, setInstalling] = useState<string | null>(null)

  const fetchPlugins = useCallback(async () => {
    setLoading(true)
    try {
      const params = new URLSearchParams({ page: String(page), limit: '20' })
      if (search) params.set('search', search)
      if (category) params.set('category', category)
      const res = await fetch(`/api/plugins?${params}`)
      const data = await res.json()
      setPlugins(data.plugins ?? [])
      setTotal(data.total ?? 0)
    } finally {
      setLoading(false)
    }
  }, [page, search, category])

  useEffect(() => {
    fetchPlugins()
  }, [fetchPlugins])

  useEffect(() => {
    // Extract unique categories
    fetch('/api/plugins?limit=100')
      .then(r => r.json())
      .then(d => {
        const cats = [...new Set((d.plugins ?? []).map((p: Plugin) => p.category).filter(Boolean))] as string[]
        setCategories(cats)
      })
      .catch(() => {})
  }, [])

  const handleInstall = async (id: string) => {
    setInstalling(id)
    try {
      await fetch(`/api/plugins/${id}/install`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ install: true }),
      })
      fetchPlugins()
    } finally {
      setInstalling(null)
    }
  }

  const totalPages = Math.ceil(total / 20)

  return (
    <div className="max-w-6xl mx-auto px-4 py-12 space-y-8">
      <PageHeader
        icon={Package}
        title="Plugin Marketplace"
        subtitle={`${total} plugin${total !== 1 ? 's' : ''} available — extend your workspace`}
      />

      {/* Search & Filter */}
      <div className="flex flex-col sm:flex-row gap-3">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground/50" />
          <input
            type="text"
            value={search}
            onChange={(e) => { setSearch(e.target.value); setPage(1) }}
            placeholder="Search plugins..."
            className="w-full rounded-lg border border-border bg-background pl-10 pr-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20"
          />
        </div>
        {categories.length > 0 && (
          <select
            value={category}
            onChange={(e) => { setCategory(e.target.value); setPage(1) }}
            className="rounded-lg border border-border bg-background px-3 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20"
          >
            <option value="">All Categories</option>
            {categories.map((c) => (
              <option key={c} value={c}>{c}</option>
            ))}
          </select>
        )}
      </div>

      {/* Plugin grid */}
      {loading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {[0, 1, 2, 3, 4, 5].map((i) => (
            <Card key={i} className="animate-pulse">
              <CardContent className="p-6 space-y-3">
                <div className="h-4 w-3/4 bg-muted rounded" />
                <div className="h-3 w-full bg-muted rounded" />
                <div className="h-3 w-2/3 bg-muted rounded" />
              </CardContent>
            </Card>
          ))}
        </div>
      ) : plugins.length === 0 ? (
        <div className="text-center py-16">
          <Package className="h-12 w-12 mx-auto text-muted-foreground/20 mb-4" />
          <p className="text-muted-foreground">No plugins found</p>
          {search && <p className="text-xs text-muted-foreground/50 mt-1">Try adjusting your search</p>}
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {plugins.map((plugin) => (
            <Card
              key={plugin.id}
              className="hover:border-primary/30 transition-colors cursor-pointer group"
              onClick={() => router.push(`/marketplace/plugins/${plugin.id}`)}
            >
              <CardContent className="p-5 space-y-3">
                {/* Header */}
                <div className="flex items-start justify-between gap-3">
                  <div className="flex items-center gap-3 min-w-0">
                    <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary text-lg font-bold">
                      {plugin.name[0]}
                    </div>
                    <div className="min-w-0">
                      <h3 className="text-sm font-semibold truncate">{plugin.name}</h3>
                      <p className="text-[10px] text-muted-foreground font-mono">v{plugin.version}</p>
                    </div>
                  </div>
                  {plugin.averageRating > 0 && (
                    <div className="flex items-center gap-1 shrink-0">
                      <Star className="h-3.5 w-3.5 fill-warning text-warning" />
                      <span className="text-xs font-medium">{plugin.averageRating.toFixed(1)}</span>
                    </div>
                  )}
                </div>

                <p className="text-xs text-muted-foreground line-clamp-2">{plugin.description}</p>

                {/* Meta */}
                <div className="flex items-center gap-2 flex-wrap">
                  <Badge variant="secondary" className="text-[10px]">{plugin.licenseType}</Badge>
                  {plugin.category && (
                    <Badge variant="outline" className="text-[10px]">{plugin.category}</Badge>
                  )}
                  <span className="text-[10px] text-muted-foreground/60">
                    <Download className="h-3 w-3 inline mr-0.5" />
                    {plugin.downloads}
                  </span>
                </div>

                {/* Actions */}
                <div className="flex items-center gap-2 pt-1" onClick={(e) => e.stopPropagation()}>
                  <Button
                    size="sm"
                    variant="outline"
                    className="gap-1.5 h-7 text-xs flex-1"
                    onClick={() => router.push(`/marketplace/plugins/${plugin.id}`)}
                  >
                    Details
                    <ArrowRight className="h-3 w-3" />
                  </Button>
                  {plugin.licenseType === 'FREE' ? (
                    <Button
                      size="sm"
                      onClick={() => handleInstall(plugin.id)}
                      disabled={installing === plugin.id}
                      className="gap-1.5 h-7 text-xs"
                    >
                      <Download className="h-3 w-3" />
                      {installing === plugin.id ? 'Installing…' : 'Install'}
                    </Button>
                  ) : (
                    <Button
                      size="sm"
                      onClick={() => router.push(`/marketplace/plugins/${plugin.id}?buy=1`)}
                      className="gap-1.5 h-7 text-xs"
                    >
                      <ShoppingCart className="h-3 w-3" />
                      ${plugin.price}
                    </Button>
                  )}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-center gap-2 pt-4">
          <Button
            size="sm"
            variant="outline"
            disabled={page <= 1}
            onClick={() => setPage(p => p - 1)}
          >
            Previous
          </Button>
          <span className="text-sm text-muted-foreground px-3">
            Page {page} of {totalPages}
          </span>
          <Button
            size="sm"
            variant="outline"
            disabled={page >= totalPages}
            onClick={() => setPage(p => p + 1)}
          >
            Next
          </Button>
        </div>
      )}
    </div>
  )
}
