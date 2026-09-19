import { CheckCircle2, FileText, Wrench } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { StreamChunk } from "@/lib/types";
import { useStore } from "@/state/store";

import { Spinner } from "./Spinner";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "./ui/accordion";

export function StreamOutput({ pipelineId }: { pipelineId: string }) {
  const { state } = useStore();
  const chunks = state.streamOutput[pipelineId] ?? [];
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chunks.length]);

  if (chunks.length === 0) {
    return <Spinner className="py-4" label="等待 Claude 输出..." />;
  }

  return (
    <div className="max-h-96 overflow-y-auto font-mono text-[13px]">
      {chunks.map((chunk, i) => (
        <ChunkLine chunk={chunk} key={i} />
      ))}
      <div ref={bottomRef} />
    </div>
  );
}

function ChunkLine({ chunk }: { chunk: StreamChunk }) {
  const [open, setOpen] = useState<string>("");

  switch (chunk.type) {
    case "text":
      return <p className="my-0.5 whitespace-pre-wrap">{chunk.content}</p>;
    case "tool_use":
      return (
        <Accordion className="mb-1" onValueChange={setOpen} type="single" value={open} collapsible>
          <AccordionItem value="tool">
            <AccordionTrigger className="text-muted-foreground py-2">
              <span className="flex items-center gap-1">
                <Wrench aria-hidden className="size-3.5" />
                {chunk.tool_name ?? "tool"}
              </span>
            </AccordionTrigger>
            <AccordionContent>
              <pre className="overflow-x-auto text-xs">{chunk.content}</pre>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      );
    case "tool_result":
      return (
        <Accordion className="mb-1" collapsible onValueChange={setOpen} type="single" value={open}>
          <AccordionItem value="result">
            <AccordionTrigger className="text-success py-2">
              <span className="flex items-center gap-1">
                <FileText aria-hidden className="size-3.5" />
                Result
              </span>
            </AccordionTrigger>
            <AccordionContent>
              <pre className="overflow-x-auto text-xs">{chunk.content}</pre>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      );
    case "result":
      return (
        <p className="text-success my-1 flex items-center gap-1 font-semibold">
          <CheckCircle2 aria-hidden className="size-3.5" />
          {chunk.content}
        </p>
      );
    default:
      return <p className="text-muted-foreground">{chunk.content}</p>;
  }
}
