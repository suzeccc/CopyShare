declare module "lucide-vue-next/dist/esm/icons/*.js" {
  import type { FunctionalComponent, SVGAttributes } from "vue";

  interface LucideProps extends Partial<SVGAttributes> {
    size?: 24 | number;
    strokeWidth?: number | string;
    absoluteStrokeWidth?: boolean;
    "absolute-stroke-width"?: boolean;
  }

  const icon: FunctionalComponent<LucideProps>;
  export default icon;
}
