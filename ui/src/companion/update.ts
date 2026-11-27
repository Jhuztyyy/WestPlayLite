import { useEffect, useState } from 'react'

export interface Offer {
  version: string
  notes: string
  date: string | null
}

export interface Progress {
  done: number
  total: number | null
}

export function useUpdateOffer(): Offer | null {
  const [offer] = useState<Offer | null>(null)
  useEffect(() => {}, [])
  return offer
}

export function lookForUpdate(): Promise<Offer | null> {
  return Promise.resolve(null)
}

export function installUpdate(): Promise<void> {
  return Promise.resolve()
}

export function onUpdateProgress(_handler: (progress: Progress) => void): () => void {
  return () => {}
}

export function noteLines(_notes: string): string[] {
  return []
}