import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from './ui/dialog';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { useConfig } from './config-provider';
import { Alert, AlertDescription } from './ui/alert';

export function ProvidePatDialog({
  open,
  onOpenChange,
  errorMessage,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  errorMessage?: string;
}) {
  const { config, updateAndSaveConfig } = useConfig();
  const [pat, setPat] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSave = async () => {
    if (!config) return;
    setSaving(true);
    setError(null);
    try {
      await updateAndSaveConfig({
        github: {
          ...config.github,
          pat,
        },
      });
      onOpenChange(false);
    } catch (err) {
      setError('Failed to save Personal Access Token');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Provide GitHub Personal Access Token</DialogTitle>
        </DialogHeader>
        <div className="space-y-2">
          <p>
            {errorMessage ||
              'Your GitHub OAuth token does not have sufficient permissions to open a PR in this repository.'}
            <br />
            <br />
            Please provide a Personal Access Token with <b>repo</b> permissions.
          </p>
          <Input
            placeholder="ghp_xxxxxxxxxxxxxxxxxxxx"
            value={pat}
            onChange={(e) => setPat(e.target.value)}
            autoFocus
          />
          <p className="text-sm text-muted-foreground">
            <a
              href="https://github.com/settings/tokens"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-600 hover:underline"
            >
              Create a token here
            </a>
          </p>
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
        </div>
        <DialogFooter>
          <Button onClick={handleSave} disabled={saving || !pat || !config}>
            {saving ? 'Saving...' : 'Save'}
          </Button>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={saving}
          >
            Cancel
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
