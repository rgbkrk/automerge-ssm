import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): State {
    return {
      hasError: true,
      error,
      errorInfo: null,
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("ErrorBoundary caught an error:", error, errorInfo);
    this.setState({
      error,
      errorInfo,
    });
  }

  copyErrorDetails = () => {
    const { error, errorInfo } = this.state;
    const errorText = `
Error: ${error?.toString()}

Component Stack:
${errorInfo?.componentStack}

Stack Trace:
${error?.stack}
    `.trim();

    navigator.clipboard.writeText(errorText);
    alert("Error details copied to clipboard!");
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen bg-background flex items-center justify-center p-4">
          <div className="max-w-2xl w-full bg-card border border-destructive rounded-lg p-6 shadow-lg">
            <div className="flex items-start gap-4">
              <div className="text-destructive text-4xl">⚠️</div>
              <div className="flex-1">
                <h1 className="text-2xl font-bold text-destructive mb-2">
                  Something went wrong
                </h1>
                <p className="text-muted-foreground mb-4">
                  The application encountered an error. You can copy the error
                  details below and share them.
                </p>

                <div className="space-y-4">
                  <div>
                    <h2 className="font-semibold mb-2">Error:</h2>
                    <pre className="bg-muted p-4 rounded text-sm overflow-x-auto">
                      {this.state.error?.toString()}
                    </pre>
                  </div>

                  {this.state.errorInfo && (
                    <div>
                      <h2 className="font-semibold mb-2">Component Stack:</h2>
                      <pre className="bg-muted p-4 rounded text-sm overflow-x-auto max-h-48 overflow-y-auto">
                        {this.state.errorInfo.componentStack}
                      </pre>
                    </div>
                  )}

                  {this.state.error?.stack && (
                    <div>
                      <h2 className="font-semibold mb-2">Stack Trace:</h2>
                      <pre className="bg-muted p-4 rounded text-sm overflow-x-auto max-h-48 overflow-y-auto">
                        {this.state.error.stack}
                      </pre>
                    </div>
                  )}

                  <div className="flex gap-2">
                    <button
                      onClick={this.copyErrorDetails}
                      className="px-4 py-2 bg-primary text-primary-foreground rounded hover:bg-primary/90"
                    >
                      Copy Error Details
                    </button>
                    <button
                      onClick={() => window.location.reload()}
                      className="px-4 py-2 bg-secondary text-secondary-foreground rounded hover:bg-secondary/80"
                    >
                      Reload Page
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
