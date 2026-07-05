import { icons, type LucideIcon, type LucideProps } from "lucide-react";

/**
 * Every lucide-react icon name is a valid `name`. We export the
 * type as `IconName` for consumers who want to constrain a prop
 * to a known icon set without losing autocomplete.
 */
export type IconName = keyof typeof icons;

export interface IconProps extends LucideProps {
  name: IconName;
}

/**
 * Thin wrapper around `lucide-react` so we can:
 *   1. Pass a string `name` (handy in serialized data/JSON).
 *   2. Render `<Icon name={source.icon} />` from planner data.
 *
 * All other props (size, color, strokeWidth, className, aria-*)
 * forward as-is to the underlying lucide component.
 */
export const Icon = ({ name, ...rest }: IconProps): JSX.Element => {
  const C: LucideIcon = icons[name];
  return <C {...rest} />;
};
