interface OutputProps {
  output: {
    outputType: "stream" | "display_data" | "execute_result" | "error";
    data: Record<string, string>;
    text?: string;
  };
}

export function Output({ output }: OutputProps) {
  if (output.outputType === "error") {
    return (
      <div className="p-3 border border-destructive rounded-md bg-destructive/10">
        <div className="text-sm font-medium text-destructive mb-2">Error:</div>
        <div className="text-sm font-mono whitespace-pre-wrap text-destructive">
          {output.text || JSON.stringify(output.data)}
        </div>
      </div>
    );
  }

  if (output.outputType === "stream") {
    return (
      <div className="p-3 border rounded-md bg-muted">
        <div className="text-sm font-mono whitespace-pre-wrap">
          {output.text || output.data["text/plain"]}
        </div>
      </div>
    );
  }

  // display_data or execute_result
  // Prioritize HTML, then images, then text
  if (output.data["text/html"]) {
    return (
      <div
        className="p-3 border rounded-md bg-background"
        dangerouslySetInnerHTML={{ __html: output.data["text/html"] }}
      />
    );
  }

  if (output.data["image/png"]) {
    return (
      <div className="p-3 border rounded-md bg-background">
        <img
          src={`data:image/png;base64,${output.data["image/png"]}`}
          alt="Output"
          className="max-w-full"
        />
      </div>
    );
  }

  if (output.data["image/jpeg"]) {
    return (
      <div className="p-3 border rounded-md bg-background">
        <img
          src={`data:image/jpeg;base64,${output.data["image/jpeg"]}`}
          alt="Output"
          className="max-w-full"
        />
      </div>
    );
  }

  // Default to text/plain
  return (
    <div className="p-3 border rounded-md bg-muted">
      <div className="text-sm font-mono whitespace-pre-wrap">
        {output.data["text/plain"] || JSON.stringify(output.data)}
      </div>
    </div>
  );
}
