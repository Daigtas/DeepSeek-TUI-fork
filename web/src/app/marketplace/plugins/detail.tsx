/**
 * Plugin Detail Page with Reviews — Phase 10 PLUGIN-02, PLUGIN-05.
 *
 * Shows plugin details, install/purchase flow, and reviews with submission.
 */

'use client'

import { useState, useEffect, useCallback } from 'react'
import { useParams, useRouter, useSearchParams } from 'next/navigation'
import { Star, Download, ShoppingCart, ArrowLeft, MessageSquare, Shield, Calendar } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Separator } from '@/components/ui/separator'

interface PluginDetail {
  id: string
  name: string
  description: string
  version: string
  author: string
  category: string
  licenseType: string
  price: number
  tags: string[]
  downloads: number
  averageRating: number
  screenshots: string[]
  repository?: string
  documentation?: string
  createdAt: string
  reviews: Review[]
  _count: { reviews: number; purchases: number }
}

interface Review {
  id: string
  userId: string
  rating: number
  comment?: string
  createdAt: string
}

export default function PluginDetailPage() {
  const params = useParams()
  const router = useRouter()
  const searchParams = useSearchParams()
  const id = params.id as string
  const [plugin, setPlugin] = useState<PluginDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [installing, setInstalling] = useState(false)
  const [buying, setBuying] = useState(false)
  const [reviewRating, setReviewRating] = useState(5)
  const [reviewComment, setReviewComment] = useState('')
  const [submittingReview, setSubmittingReview] = useState(false)
  const [reviewError, setReviewError] = useState('')
  const [reviewSuccess, setReviewSuccess] = useState('')

  const fetchPlugin = useCallback(async () => {
    try {
      const res = await fetch(`/api/plugins/${id}`)
      const data = await res.json()
      setPlugin(data.plugin)
    } finally {
      setLoading(false)
    }
  }, [id])

  useEffect(() => {
    fetchPlugin()
  }, [fetchPlugin])

  const handleInstall = async () => {
    setInstalling(true)
    try {
      await fetch(`/api/plugins/${id}/install`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ install: true }),
      })
      fetchPlugin()
    } finally {
      setInstalling(false)
    }
  }

  const handlePurchase = async () => {
    setBuying(true)
    try {
      const res = await fetch(`/api/plugins/${id}/purchase`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      })
      if (res.ok) {
        const data = await res.json()
        if (data.url) {
          window.location.href = data.url
          return
        }
      }
      fetchPlugin()
    } finally {
      setBuying(false)
    }
  }

  const handleSubmitReview = async () => {
    setSubmittingReview(true)
    setReviewError('')
    setReviewSuccess('')
    try {
      const res = await fetch(`/api/plugins/${id}/reviews`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ rating: reviewRating, comment: reviewComment || undefined }),
      })
      if (!res.ok) {
        const err = await res.json()
        setReviewError(err.error ?? 'Failed to submit review')
        return
      }
      setReviewSuccess('Review submitted!')
      setReviewComment('')
      setReviewRating(5)
      fetchPlugin()
    } finally {
      setSubmittingReview(false)
    }
  }

  if (loading) {
    return (
      <div className="max-w-4xl mx-auto px-4 py-12">
        <div className="animate-pulse space-y-4">
          <div className="h-8 w-1/3 bg-muted rounded" />
          <div className="h-4 w-2/3 bg-muted rounded" />
          <div className="h-48 bg-muted rounded-xl" />
        </div>
      </div>
    )
  }

  if (!plugin) {
    return (
      <div className="max-w-4xl mx-auto px-4 py-12 text-center">
        <p className="text-muted-foreground">Plugin not found</p>
        <Button variant="outline" className="mt-4" onClick={() => router.back()}>
          <ArrowLeft className="h-4 w-4 mr-2" /> Go Back
        </Button>
      </div>
    )
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-12 space-y-8">
      {/* Back navigation */}
      <button
        onClick={() => router.back()}
        className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
      >
        <ArrowLeft className="h-4 w-4" /> Back to Marketplace
      </button>

      {/* Hero */}
      <div className="flex flex-col sm:flex-row gap-6">
        <div className="flex h-20 w-20 shrink-0 items-center justify-center rounded-2xl bg-primary/10 text-primary text-3xl font-bold">
          {plugin.name[0]}
        </div>
        <div className="flex-1 space-y-3">
          <div>
            <h1 className="text-2xl font-bold">{plugin.name}</h1>
            <p className="text-sm text-muted-foreground mt-1">
              by {plugin.author} · v{plugin.version}
            </p>
          </div>
          <div className="flex items-center gap-3 flex-wrap">
            {plugin.averageRating > 0 && (
              <div className="flex items-center gap-1">
                {[1, 2, 3, 4, 5].map((s) => (
                  <Star
                    key={s}
                    className={`h-4 w-4 ${s <= Math.round(plugin.averageRating) ? 'fill-warning text-warning' : 'text-muted-foreground/30'}`}
                  />
                ))}
                <span className="text-sm font-medium ml-1">{plugin.averageRating.toFixed(1)}</span>
                <span className="text-xs text-muted-foreground">({plugin._count.reviews})</span>
              </div>
            )}
            <Badge>{plugin.licenseType}</Badge>
            {plugin.category && <Badge variant="outline">{plugin.category}</Badge>}
          </div>
          <div className="flex items-center gap-4 text-xs text-muted-foreground">
            <span className="flex items-center gap-1">
              <Download className="h-3 w-3" /> {plugin.downloads} downloads
            </span>
            <span className="flex items-center gap-1">
              <Calendar className="h-3 w-3" /> {new Date(plugin.createdAt).toLocaleDateString()}
            </span>
          </div>
        </div>

        {/* Action buttons */}
        <div className="flex flex-col gap-2 min-w-[160px]">
          {plugin.licenseType === 'FREE' ? (
            <Button onClick={handleInstall} disabled={installing} className="gap-2">
              <Download className="h-4 w-4" />
              {installing ? 'Installing…' : 'Install Free'}
            </Button>
          ) : (
            <Button onClick={handlePurchase} disabled={buying} className="gap-2">
              <ShoppingCart className="h-4 w-4" />
              {buying ? 'Processing…' : `Buy $${plugin.price}`}
            </Button>
          )}
        </div>
      </div>

      {/* Description */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-semibold">About</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground whitespace-pre-wrap">{plugin.description}</p>

          {(plugin.repository || plugin.documentation) && (
            <div className="flex items-center gap-4 mt-4">
              {plugin.repository && (
                <a href={plugin.repository} target="_blank" rel="noopener noreferrer" className="text-xs text-primary hover:underline">
                  View Repository →
                </a>
              )}
              {plugin.documentation && (
                <a href={plugin.documentation} target="_blank" rel="noopener noreferrer" className="text-xs text-primary hover:underline">
                  Documentation →
                </a>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Tags */}
      {plugin.tags.length > 0 && (
        <div className="flex items-center gap-2 flex-wrap">
          {plugin.tags.map((tag) => (
            <Badge key={tag} variant="secondary" className="text-[10px]">{tag}</Badge>
          ))}
        </div>
      )}

      <Separator />

      {/* Reviews */}
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <MessageSquare className="h-5 w-5" />
            Reviews ({plugin._count.reviews})
          </h2>
        </div>

        {/* Submit review */}
        <Card>
          <CardContent className="p-4 space-y-3">
            <h3 className="text-sm font-semibold">Write a Review</h3>
            <div className="flex items-center gap-1">
              {[1, 2, 3, 4, 5].map((s) => (
                <button
                  key={s}
                  onClick={() => setReviewRating(s)}
                  className="focus:outline-none"
                >
                  <Star
                    className={`h-5 w-5 transition-colors ${s <= reviewRating ? 'fill-warning text-warning' : 'text-muted-foreground/30 hover:text-warning/50'}`}
                  />
                </button>
              ))}
            </div>
            <textarea
              value={reviewComment}
              onChange={(e) => setReviewComment(e.target.value)}
              placeholder="Share your experience with this plugin…"
              className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/20 resize-none h-20"
            />
            <div className="flex items-center gap-3">
              <Button
                size="sm"
                onClick={handleSubmitReview}
                disabled={submittingReview}
                className="gap-1.5"
              >
                {submittingReview ? 'Submitting…' : 'Submit Review'}
              </Button>
              {reviewError && <p className="text-xs text-destructive">{reviewError}</p>}
              {reviewSuccess && <p className="text-xs text-success">{reviewSuccess}</p>}
            </div>
          </CardContent>
        </Card>

        {/* Reviews list */}
        {plugin.reviews.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-8">No reviews yet. Be the first!</p>
        ) : (
          <div className="space-y-3">
            {plugin.reviews.map((review) => (
              <Card key={review.id}>
                <CardContent className="p-4 space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-1">
                      {[1, 2, 3, 4, 5].map((s) => (
                        <Star
                          key={s}
                          className={`h-3.5 w-3.5 ${s <= review.rating ? 'fill-warning text-warning' : 'text-muted-foreground/20'}`}
                        />
                      ))}
                    </div>
                    <span className="text-[10px] text-muted-foreground">
                      {new Date(review.createdAt).toLocaleDateString()}
                    </span>
                  </div>
                  {review.comment && (
                    <p className="text-sm text-muted-foreground">{review.comment}</p>
                  )}
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
