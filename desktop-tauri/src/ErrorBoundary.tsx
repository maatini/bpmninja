import { Component } from 'react';
import type { ReactNode, ErrorInfo } from 'react';
import { AlertTriangle, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardContent, CardFooter } from '@/components/ui/card';

interface Props {
  children?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
    errorInfo: null
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error, errorInfo: null };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Uncaught error:', error, errorInfo);
    this.setState({ error, errorInfo });
  }

  public render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen bg-background flex flex-col items-center justify-center p-6 text-foreground">
          <Card className="w-full max-w-2xl border-destructive/50 bg-destructive/5 shadow-lg">
            <CardHeader className="border-b bg-destructive/10">
              <CardTitle className="text-xl flex items-center gap-2 text-destructive font-mono">
                <AlertTriangle className="h-6 w-6" />
                Application Render Error
              </CardTitle>
            </CardHeader>
            <CardContent className="pt-6 space-y-4">
              <p className="text-sm font-medium">An unexpected error occurred while rendering the interface.</p>
              
              <div className="bg-destructive/10 p-4 rounded-md border border-destructive/20 text-sm overflow-x-auto font-mono text-destructive">
                {this.state.error && this.state.error.toString()}
              </div>
              
              {this.state.errorInfo && (
                <div>
                  <h4 className="text-xs font-bold uppercase tracking-wider text-muted-foreground mb-2">Component Stack Trace</h4>
                  <pre className="bg-muted border border-input text-foreground p-4 rounded-md text-xs overflow-x-auto whitespace-pre-wrap font-mono">
                    {this.state.errorInfo.componentStack}
                  </pre>
                </div>
              )}
            </CardContent>
            <CardFooter className="pt-2 flex justify-end gap-3 mt-4">
               <Button onClick={() => window.location.reload()} variant="outline" className="gap-2">
                 <RefreshCw className="h-4 w-4" /> Reload Application
               </Button>
            </CardFooter>
          </Card>
        </div>
      );
    }

    return this.props.children;
  }
}
