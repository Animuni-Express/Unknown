import { Playground } from "@/components/playground";
export const metadata = {
  title: "Playground",
  description:
    "Execute real Unknown and USH programs with a shared virtual filesystem.",
  alternates: {
    canonical: `${process.env.NEXT_PUBLIC_BASE_PATH || ""}/playground/`,
  },
};
export default function Page() {
  return <Playground />;
}
