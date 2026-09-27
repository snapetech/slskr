import React from 'react';

class ErrorBoundary extends React.Component {
  state = { hasError: false };

  static getDerivedStateFromError() {
    return { hasError: true };
  }

  componentDidCatch(error, errorInfo) {
    if (typeof console !== 'undefined' && console.error) {
      console.error('Web application render failed', error, errorInfo);
    }
  }

  handleReload = () => {
    if (typeof window !== 'undefined' && window.location?.reload) {
      window.location.reload();
    }
  };

  render() {
    if (!this.state.hasError) return this.props.children;

    return (
      <main role="alert" className="app-error-boundary">
        <h1>Something went wrong</h1>
        <p>The application could not render this view.</p>
        <button type="button" onClick={this.handleReload}>
          Reload application
        </button>
      </main>
    );
  }
}

export default ErrorBoundary;
