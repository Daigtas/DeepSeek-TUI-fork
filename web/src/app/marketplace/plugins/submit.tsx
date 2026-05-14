/**
 * Developer Plugin Submission Page — Phase 10 PLUGIN-04.
 *
 * Form at /developer/plugins/submit for developers to submit plugins
 * to the marketplace for review.
 */

'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { ArrowLeft, Upload, Code, Tag, DollarSign } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { PageHeader } from '@/components/ui/molecules'
import { Separator } from '@/components/ui/separator'

export default function PluginSubmitPage() {
  const router = useRouter()
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState('')
  const [success, setSuccess] = useState(false)

  const [form, setForm] = useState({
    name: '',
    slug: '',
    version: '1.0.0',
    description: '',
    category: '',
    licenseType: 'FREE' as const,
    price: 0,
    tags: '',
    repository: '',
    documentation: '',
  })

  const handleChange = (field: string, value: string | number) => {
    setForm((prev) => ({ ...prev, [field]: value }))
  }

  const generateSlug = () => {
    const slug = form.name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
    setForm((prev) => ({ ...prev, slug }))
  }

  const handleSubmit = async () => {
    setError('')
    setSubmitting(true)

    try {
      const tags = form.tags
        ? form.tags.split(',').map((t) => t.trim()).filter(Boolean)
        : []

      const res = await fetch('/api/plugins', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: form.name,
          slug: form.slug || form.name.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
          version: form.version,
          description: form.description,
          category: form.category,
          licenseType: form.licenseType,
          price: form.licenseType === 'FREE' ? 0 : form.price,
          tags,
          repository: form.repository || undefined,
          documentation: form.documentation || undefined,
        }),
      })

      if (!res.ok) {
        const err = await res.json()
        setError(err.error?.fieldErrors
          ? Object.entries(err.error.fieldErrors).map(([k, v]) => `${k}: ${(v as string[]).join(', ')}`).join('; ')
          : err.error ?? 'Submission failed')
        return
      }

      setSuccess(true)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Submission failed')
    } finally {
      setSubmitting(false)
    }
  }

  if (success) {
    return (
      <div className="max-w-2xl mx-auto px-4 py-16 text-center space-y-4">
        <div className="flex items-center justify-center h-16 w-16 rounded-full bg-success/10 mx-auto">
          <Upload className="h-8 w-8 text-success" />
        </div>
        <h1 className="text-2xl font-bold">Plugin Submitted!</h1>
        <p className="text-muted-foreground">
          Your plugin has been submitted for review. You&apos;ll be notified when it&apos;s approved.
        </p>
        <div className="flex items-center justify-center gap-3 pt-4">
          <Button variant="outline" onClick={() => router.push('/marketplace/plugins')}>
            Browse Marketplace
          </Button>
          <Button onClick={() => router.push('/developer/plugins')}>
            My Plugins
          </Button>
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-2xl mx-auto px-4 py-12 space-y-6">
      <button
        onClick={() => router.back()}
        className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
      >
        <ArrowLeft className="h-4 w-4" /> Back
      </button>

      <PageHeader
        icon={Code}
        title="Submit a Plugin"
        subtitle="Share your plugin with the ForgeHub community. All submissions are reviewed before publishing."
      />

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-semibold">Plugin Details</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Name */}
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1.5 block">Plugin Name *</label>
            <input
              type="text"
              value={form.name}
              onChange={(e) => handleChange('name', e.target.value)}
              onBlur={generateSlug}
              placeholder="My Awesome Plugin"
              className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20"
              required
            />
          </div>

          {/* Slug */}
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1.5 block">Slug *</label>
            <input
              type="text"
              value={form.slug}
              onChange={(e) => handleChange('slug', e.target.value)}
              placeholder="my-awesome-plugin"
              className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-primary/20"
              required
            />
          </div>

          {/* Version */}
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1.5 block">Version *</label>
            <input
              type="text"
              value={form.version}
              onChange={(e) => handleChange('version', e.target.value)}
              placeholder="1.0.0"
              className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-primary/20"
              required
            />
          </div>

          {/* Description */}
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1.5 block">Description *</label>
            <textarea
              value={form.description}
              onChange={(e) => handleChange('description', e.target.value)}
              placeholder="Describe what your plugin does, its features, and how to use it..."
              className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20 resize-none h-28"
              required
            />
          </div>

          {/* Category */}
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1.5 block">Category</label>
            <input
              type="text"
              value={form.category}
              onChange={(e) => handleChange('category', e.target.value)}
              placeholder="e.g. UI, Analytics, Workflow, Security"
              className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20"
            />
          </div>

          <Separator />

          {/* License + Price */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="text-xs font-medium text-muted-foreground mb-1.5 block">License Type</label>
              <select
                value={form.licenseType}
                onChange={(e) => handleChange('licenseType', e.target.value)}
                className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20"
              >
                <option value="FREE">Free</option>
                <option value="PAID">Paid</option>
                <option value="FREEMIUM">Freemium</option>
                <option value="TRIAL">Trial</option>
              </select>
            </div>
            {form.licenseType !== 'FREE' && (
              <div>
                <label className="text-xs font-medium text-muted-foreground mb-1.5 block">Price (USD)</label>
                <div className="relative">
                  <DollarSign className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground/50" />
                  <input
                    type="number"
                    min={0}
                    step={0.01}
                    value={form.price}
                    onChange={(e) => handleChange('price', parseFloat(e.target.value) || 0)}
                    className="w-full rounded-lg border border-border bg-background pl-9 pr-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20"
                  />
                </div>
              </div>
            )}
          </div>

          {/* Tags */}
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1.5 block flex items-center gap-1.5">
              <Tag className="h-3.5 w-3.5" /> Tags
            </label>
            <input
              type="text"
              value={form.tags}
              onChange={(e) => handleChange('tags', e.target.value)}
              placeholder="Comma-separated, e.g. analytics, dashboard, reports"
              className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20"
            />
          </div>

          <Separator />

          {/* Repository */}
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1.5 block">Repository URL</label>
            <input
              type="url"
              value={form.repository}
              onChange={(e) => handleChange('repository', e.target.value)}
              placeholder="https://github.com/username/repo"
              className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-primary/20"
            />
          </div>

          {/* Documentation */}
          <div>
            <label className="text-xs font-medium text-muted-foreground mb-1.5 block">Documentation URL</label>
            <input
              type="url"
              value={form.documentation}
              onChange={(e) => handleChange('documentation', e.target.value)}
              placeholder="https://docs.example.com"
              className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-primary/20"
            />
          </div>

          {error && (
            <div className="p-3 rounded-lg bg-destructive/10 border border-destructive/20 text-sm text-destructive">
              {error}
            </div>
          )}

          <Button
            onClick={handleSubmit}
            disabled={submitting || !form.name || !form.description}
            className="w-full gap-2"
          >
            <Upload className="h-4 w-4" />
            {submitting ? 'Submitting…' : 'Submit for Review'}
          </Button>
        </CardContent>
      </Card>
    </div>
  )
}
