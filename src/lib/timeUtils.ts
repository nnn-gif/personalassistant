/**
 * Formats minutes into a human-readable time string
 * @param minutes Total minutes
 * @returns Formatted time string (e.g., "1h 30m", "45m", "2h")
 */
export function formatTime(minutes: number): string {
  if (minutes < 1) {
    return '0m'
  }
  
  const hours = Math.floor(minutes / 60)
  const remainingMinutes = minutes % 60
  
  if (hours === 0) {
    return `${remainingMinutes}m`
  }
  
  if (remainingMinutes === 0) {
    return `${hours}h`
  }
  
  return `${hours}h ${remainingMinutes}m`
}

/**
 * Formats decimal hours into a human-readable time string
 * @param hours Decimal hours (e.g., 1.5 = 1 hour 30 minutes)
 * @returns Formatted time string (e.g., "1h 30m", "45m", "2h")
 */
export function formatDecimalHours(hours: number): string {
  const totalMinutes = Math.round(hours * 60)
  return formatTime(totalMinutes)
}