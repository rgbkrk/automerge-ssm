import { useState, useEffect } from "react";
import { DocHandle } from "@automerge/automerge-repo";
import { ImmutableString } from "@automerge/automerge";
import { AutomergeCodeMirror } from "./AutomergeCodeMirror";
import { Output } from "./Output";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Play, Trash2, ArrowUp, ArrowDown } from "lucide-react";

interface NotebookCellProps {
  docHandle: DocHandle<any>;
  cellIndex: number;
  cell: {
    id: ImmutableString | string;
    cellType: ImmutableString | string;
    source: ImmutableString | string;
    executionCount: number | null;
    outputRefs: (ImmutableString | string)[];
  };
  onExecute: (index: number) => void;
  onDelete: (index: number) => void;
  onMoveUp: (index: number) => void;
  onMoveDown: (index: number) => void;
  isFirst: boolean;
  isLast: boolean;
}

export function NotebookCell({
  docHandle,
  cellIndex,
  cell,
  onExecute,
  onDelete,
  onMoveUp,
  onMoveDown,
  isFirst,
  isLast,
}: NotebookCellProps) {
  const [outputs, setOutputs] = useState<any[]>([]);
  const [isExecuting, setIsExecuting] = useState(false);

  const cellType = typeof cell.cellType === "string"
    ? cell.cellType
    : cell.cellType.toString();

  const executionCount = cell.executionCount;

  // Load outputs from outputRefs
  useEffect(() => {
    const loadOutputs = async () => {
      if (!cell.outputRefs || cell.outputRefs.length === 0) {
        setOutputs([]);
        return;
      }

      // Load outputs from refs
      const loadedOutputs = await Promise.all(
        cell.outputRefs.map(async (ref) => {
          const refStr = typeof ref === "string" ? ref : ref.toString();

          // Handle hokey:// protocol - convert to file path
          if (refStr.startsWith("hokey://localhost/outputs/")) {
            const outputId = refStr.replace("hokey://localhost/outputs/", "");

            try {
              // Try to fetch from local outputs directory
              // In development, Vite can serve these with proper config
              // For now, we'll create a readable message
              return {
                outputType: "execute_result" as const,
                data: {
                  "text/plain": `📦 Output stored at: ${refStr}\n\nTo view full output, check:\n./outputs/${outputId}.json\n\n(Kernel executed this cell!)`,
                },
              };
            } catch (e) {
              return {
                outputType: "error" as const,
                data: {
                  "text/plain": `Failed to load output: ${refStr}`,
                },
              };
            }
          }

          // For non-hokey URLs, show the ref
          return {
            outputType: "execute_result" as const,
            data: {
              "text/plain": `Output ref: ${refStr}`,
            },
          };
        })
      );

      setOutputs(loadedOutputs);
    };

    loadOutputs();
  }, [cell.outputRefs]);

  const handleExecute = async () => {
    setIsExecuting(true);
    await onExecute(cellIndex);
    setIsExecuting(false);
  };

  return (
    <Card className="mb-4">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Badge variant={cellType === "code" ? "default" : "secondary"}>
              {cellType === "code" ? "Code" : "Markdown"}
            </Badge>
            {cellType === "code" && executionCount !== null && (
              <Badge variant="outline">[{executionCount}]</Badge>
            )}
          </div>
          <div className="flex gap-1">
            {cellType === "code" && (
              <Button
                size="sm"
                variant="ghost"
                onClick={handleExecute}
                disabled={isExecuting}
              >
                <Play className="h-4 w-4" />
              </Button>
            )}
            <Button
              size="sm"
              variant="ghost"
              onClick={() => onMoveUp(cellIndex)}
              disabled={isFirst}
            >
              <ArrowUp className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => onMoveDown(cellIndex)}
              disabled={isLast}
            >
              <ArrowDown className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => onDelete(cellIndex)}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <AutomergeCodeMirror
          docHandle={docHandle}
          path={["cells", String(cellIndex), "source"]}
          language={cellType === "code" ? "javascript" : "markdown"}
        />

        {/* Output display */}
        {outputs.length > 0 && (
          <div className="mt-4 space-y-2">
            {outputs.map((output, idx) => (
              <Output key={idx} output={output} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
